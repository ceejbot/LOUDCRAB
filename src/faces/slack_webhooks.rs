use slack_api as slack;
use slack::chat::PostMessageRequest;
use crate::Loudbot;

pub struct LoudbotSlack {
    /// the API token we must send to Slack
    slack_token: String,
    /// the verification token Slack must send to us
    pub verification: String,
    /// our loudbot brain
    brain: Loudbot
}

impl LoudbotSlack {
    pub fn new(slack_token: String, verification: String, brain: Loudbot) -> Self {
        LoudbotSlack { slack_token, verification, brain }
    }

    // Given data about an incoming request, verify that it came from Slack.
    pub async fn verify_request() -> anyhow::Result<bool> {
        // TODO unimplemented
        Ok(false)
    }

    /// If we have a welcome channel, send a toast to it.
    pub async fn maybe_toast(&self) -> anyhow::Result<bool> {
        if let Ok(toast) = std::env::var("WELCOME_CHANNEL") {
            self.send_message(&toast, "THIS LOUDBOT IS NOW SCUTTLING", None)
                .await
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
                    let retort = self.brain.process(text).await;
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
    pub async fn yell(&self, prompt: &slack::MessageStandard, retort: &str) -> anyhow::Result<bool> {
        let channel = prompt.channel.as_ref().unwrap();
        log::info!(
            "yelling: `{retort}`; prompt: `{}`' channel: `{channel}`",
            prompt.text.as_ref().unwrap()
        );
        let sent = self.send_message(channel, retort, prompt.thread_ts).await?;
        self.brain.increment_yells().await;
        Ok(sent)
    }

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
