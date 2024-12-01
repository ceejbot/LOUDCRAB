//! The traditional implementation with webhooks & rtm events. Slack after
//! demanding for years that we use this, has now retired it. Sigh.

use async_trait::async_trait;
use slack::{chat::PostMessageRequest, Message};
use slack_api::{self as slack};

use crate::Loudbot;

use crate::IsLoudbotIntegration;

#[derive(Debug, Clone)]
pub struct LoudHooks {
    /// the API token we must send to Slack
    slack_token: String,
    /// the verification token Slack must send to us
    pub verification: String,
    /// our loudbot brain
    brain: Loudbot,
}

impl LoudHooks {
    /// Slack implementation: send a message
    async fn send_message(
        &self,
        channel: &str,
        text: &str,
        maybe_ts: Option<slack::Timestamp>,
    ) -> Result<bool, anyhow::Error> {
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
}

#[async_trait]
impl IsLoudbotIntegration for LoudHooks {
    type Msg = slack::Message;

    fn create(brain: Loudbot) -> Self {
        // crashing is appropriate if these are missing
        let slack_token =
            std::env::var("SLACK_TOKEN").expect("You must provide a valid slack api token in the env var SLACK_TOKEN.");
        let verification = std::env::var("VERIFICATION_TOKEN")
            .expect("You must provide your slack verification token in the env var VERIFICATION_TOKEN.");

        LoudHooks {
            slack_token,
            verification,
            brain,
        }
    }

    async fn verify_request() -> anyhow::Result<bool> {
        // TODO unimplemented
        Ok(false)
    }

    async fn maybe_toast(&self) -> anyhow::Result<bool> {
        if let Ok(toast) = std::env::var("WELCOME_CHANNEL") {
            self.send_message(&toast, "THIS LOUDBOT IS NOW SCUTTLING", None).await
        } else {
            Ok(false)
        }
    }

    async fn handle_message(&self, incoming: &Message) -> anyhow::Result<bool> {
        match incoming {
            Message::BotMessage(ref _y) => {
                log::debug!("skipping bot message");
                Ok(false)
            }
            Message::Standard(ref prompt) => {
                if let Some(_bot_id) = &prompt.bot_id {
                    log::info!("skipping bot message");
                    Ok(false)
                } else if prompt.text.is_none() || prompt.channel.is_none() {
                    Ok(false) // nothing to be done
                } else {
                    let text = prompt.text.as_ref().unwrap(); // we know this is safe
                    let retort = self.brain.process(text).await;
                    if let Some(r) = retort {
                        self.yell(incoming, &r).await
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

    async fn yell(&self, incoming: &Message, retort: &str) -> anyhow::Result<bool> {
        let Message::Standard(prompt) = incoming else {
            return Err(anyhow::anyhow!("did not send a slack message enum to yell()"));
        };

        let channel = prompt.channel.as_ref().unwrap();
        log::info!(
            "yelling: `{retort}`; prompt: `{}`' channel: `{channel}`",
            prompt.text.as_ref().unwrap()
        );
        let sent = self.send_message(channel, retort, prompt.thread_ts).await?;
        self.brain.increment_yells().await;
        Ok(sent)
    }
}
