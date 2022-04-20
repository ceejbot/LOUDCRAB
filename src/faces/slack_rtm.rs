use slack::{Event, RtmClient};
use slack_api::Message;
use tokio::runtime::Runtime;

use crate::Loudbot;

pub struct LoudbotRTM {
    /// the API token we must send to Slack
    slack_token: String,
    /// our loudbot brain
    brain: Loudbot,
    rt: Runtime,
}

impl slack::EventHandler for LoudbotRTM {
    fn on_event(&mut self, _cli: &RtmClient, event: Event) {
        match event {
            Event::Hello => {
                self.rt.block_on(async {
                    let _ = self.maybe_toast().await;
                });
            },
            Event::Message(ref prompt) => {
                // Create the runtime
                self.rt.block_on(async {
                    let _ = self.handle_message(prompt).await;
                });
            },
            Event::MessageSent(_) => {},
            _ => log::debug!("on_event(event: {:?})", event),
        };
    }

    fn on_close(&mut self, _cli: &RtmClient) {
        log::warn!("on_close; loudie has no idea what to do here yet");
        // TODO reconnect
    }

    fn on_connect(&mut self, _cli: &RtmClient) {
        log::info!("THIS BATTLESTATION WILL BE FULLY OPERATIONAL SHORTLY");
    }
}

impl LoudbotRTM {
    pub fn new(slack_token: String, brain: Loudbot) -> Self {
        let rt  = Runtime::new().unwrap();

        LoudbotRTM { slack_token, brain, rt }
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
    pub async fn handle_message(&self, incoming: &Message) -> anyhow::Result<bool> {
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
    pub async fn yell(
        &self,
        prompt: &slack_api::MessageStandard,
        retort: &str,
    ) -> anyhow::Result<bool> {
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
        maybe_ts: Option<slack_api::Timestamp>,
    ) -> Result<bool, anyhow::Error> {
        let message = slack_api::chat::PostMessageRequest {
            channel,
            text,
            thread_ts: maybe_ts,
            unfurl_links: Some(true),
            link_names: Some(true),
            ..slack_api::chat::PostMessageRequest::default()
        };

        let client = slack_api::default_client()?;
        let response = slack_api::chat::post_message(&client, &self.slack_token, &message).await;
        match response {
            Err(e) => {
                log::error!("error trying to post message: {:?}", e);
                Err(anyhow::anyhow!(e))
            }
            Ok(_) => Ok(true),
        }
    }
}
