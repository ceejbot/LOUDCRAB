use anyhow::{Context, Error, Result};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use slack::chat::PostMessageRequest;
use slack_api as slack;

use std::convert::AsRef;

use super::classifier::*;

type RString = std::result::Result<String, redis::RedisError>;

/// The LOUDBOT struct (sadly not shoutcased) is our app state.
///
/// This structure holds the slack response information as well as the redis
/// connections: anything we want to live through the whole process.
#[derive(Clone)]
pub struct Loudbot {
    /// the API token we must send to Slack
    slack_token: String,
    /// the verification token Slack must send to us
    pub verification: String,
    /// our redis connection
    db: MultiplexedConnection,
    /// the message classifier
    detector: Classifier,
}

impl Loudbot {
    pub async fn new(
        slack_token: String,
        verification: String,
        redis_uri: String,
        malc_chance: u8,
    ) -> Result<Loudbot, anyhow::Error> {
        let client = redis::Client::open(redis_uri.as_ref())
            .with_context(|| format!("Unable to create redis client @ {}", redis_uri))?;
        let db = client.get_multiplexed_async_std_connection().await?;

        let detector = Classifier::new(malc_chance);

        Ok(Loudbot {
            slack_token,
            verification,
            db,
            detector,
        })
    }

    pub async fn random_yell(&self) -> Option<String> {
        self.select(YELLS).await
    }

    /// If we have a welcome channel, send a toast to it.
    pub async fn maybe_toast(&self) -> anyhow::Result<bool> {
        if let Ok(toast) = std::env::var("WELCOME_CHANNEL") {
            self.send_message(&toast, "THIS LOUDBOT IS NOW SCUTTLING", None).await
        } else {
            Ok(false)
        }
    }

    /// Process an incoming message from slack and make decisions based on its envelope.
    /// Slack-specific
    pub async fn handle_message(&self, incoming: slack::Message) -> anyhow::Result<bool> {
        match incoming {
            slack::Message::BotMessage(ref _y) => {
                log::debug!("skipping bot message");
                Ok(false)
            }
            slack::Message::Standard(ref prompt) => {
                if let Some(_bot_id) = &prompt.bot_id {
                    log::info!("skipping bot message");
                    Ok(false)
                } else if prompt.text.is_none() || prompt.channel.is_none() {
                   Ok(false) // nothing to be done
                } else {
                    let text = prompt.text.as_ref().unwrap(); // we know this is safe
                    let retort = self.process(text).await;
                    if let Some(r) = retort {
                        self.yell(prompt, &r).await
                    } else {
                        Ok(false)
                    }
                }
            }
            _ => {
                // we're just ignoring it
                Ok(false)
            }
        }
    }

    /// Post a yell and record that we're doing so. Prefer this function to yell.
    async fn yell(&self, prompt: &slack::MessageStandard, retort: &str) -> anyhow::Result<bool> {
        let channel = prompt.channel.as_ref().unwrap();
        log::info!(
            "yelling: `{retort}`; prompt: `{}`' channel: `{channel}`",
            prompt.text.as_ref().unwrap()
        );
        let sent = self.send_message(channel, retort, prompt.thread_ts).await?;
        self.increment(COUNT).await; // todo use of redis key directly
        Ok(sent)
    }

    /// Slack implementation: send a message
    pub async fn send_message(
        &self,
        channel: &str,
        text: &str,
        maybe_ts: Option<slack::Timestamp>,
    ) -> Result<bool, Error> {
        let message = PostMessageRequest {
            channel,
            text,
            thread_ts: maybe_ts,
            unfurl_links: Some(true),
            link_names: Some(true),
            ..PostMessageRequest::default()
        };

        let client = slack::default_client()?;
        let response = slack::chat::post_message(&client, &self.slack_token, &message).await;
        match response {
            Err(e) => {
                log::error!("error trying to post message: {:?}", e);
                Err(anyhow::anyhow!(e))
            }
            Ok(_) => Ok(true),
        }
    }

    /// Examine a text string and decide if we want to retort.
    async fn process(&self, text: &str) -> Option<String> {
        let retort = match self.detector.classify(text) {
            Retort::None => None,
            Retort::Canned(r) => Some(r),
            Retort::Random(set) => {
                // Every named set of responses has a corresponding counter.
                let counter = format!("{set}_COUNT");
                self.increment(&counter).await;
                self.select(&set).await
            }
            Retort::Report => self.report().await,
            Retort::Remember(set) => {
                let yell = self.select(&set).await;
                self.remember(&set, text).await;
                yell
            }
        };

        retort
    }

    /// Increment the named counter, ignoring errors because this is a nice-to-have not a requirement.
    async fn increment(&self, counter: &str) {
        let mut r = self.db.clone();
        let _ = r.incr::<&str, u32, u32>(counter, 1_u32).await;
    }

    /// LOUDBOT REMEMBERS WHAT YOU SHOUT.
    async fn remember(&self, key: &str, shout: &str) {
        let mut r = self.db.clone();
        let _ = r.sadd::<&str, &str, u32>(key, shout).await;
    }

    /// Select a random message from the named message set.
    pub async fn select(&self, key: &str) -> Option<String> {
        let mut r = self.db.clone();
        let retort: RString = r.srandmember(key).await;
        match retort {
            Err(e) => {
                log::warn!("Failed to get a random set member from redis: {:?}", e);
                None
            }
            Ok(retort) => Some(retort.to_uppercase()),
        }
    }

    /// Respond to the `report` command. This is the only remaining place
    /// that needs specific redis key strings.
    async fn report(&self) -> Option<String> {
        let mut r = self.db.clone();

        let count = match r.get::<&str, String>(COUNT).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let cardinality = match r.scard::<&str, u32>(YELLS).await {
            Ok(c) => c.to_string(),
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let key = format!("{MALCOLM}_COUNT");
        let malcolms = match r.get::<&str, String>(&key).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let version = env!("CARGO_PKG_VERSION");
        Some(format!("I AM RUNNING LOUDOS VERSION {version}. I HAVE YELLED {count} TIMES. I HAVE {cardinality} THINGS TO YELL AT YOU. MALCOLM TUCKER HAS BEEN SUMMONED {malcolms} TIMES."))
    }

}
