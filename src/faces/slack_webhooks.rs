pub struct SlackFace {
    verification: String,
    slack_token: String,
}

impl LoudbotFace for SlackFace {
    /// Process an incoming message from slack and make decisions based on its envelope.
    async fn handle_message(&self, incoming: Message) -> anyhow::Result<bool> {
        match incoming {
            Message::BotMessage(ref _y) => {
                log::debug!("skipping bot message");
                Ok(false)
            }
            Message::Standard(ref x) => {
                if let Some(_bot_id) = &x.bot_id {
                    log::info!("skipping bot message");
                    Ok(false)
                } else {
                    self.process(x).await
                }
            }
            _ => {
                // we're just ignoring it
                Ok(false)
            }
        }
    }

    async fn verify_message() -> anyhow::Result<bool> {
        false
    }

    /// Internal implementation of sending a message to Slack.
    async fn send_message(
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
}
