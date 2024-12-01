//! The traditional implementation with webhooks & rtm events. Slack after
//! demanding for years that we use this, has now retired it. Sigh.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;
use slack::{chat::PostMessageRequest, Message};
use slack_api::{self as slack};

use crate::IsLoudbotIntegration;
use crate::Loudbot;
use crate::LoudbotFace;

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

/// The parts of an incoming Slack webhook post that we care about.
#[derive(Clone, Deserialize, Debug)]
struct IncomingEvent {
    /// Verification token, which must match what we expect.
    token: String,
    /// Type of the incoming message event.
    #[serde(rename = "type")]
    message_type: Option<String>,
    /// Full event payload.
    event: Option<<LoudHooks as IsLoudbotIntegration>::Msg>,
    /// The remainder of the envelope, which is only needed sometimes.
    #[serde(flatten)]
    rest: HashMap<String, serde_json::Value>,
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

    fn routes(prefix: &str) -> Router<Arc<LoudHooks>> {
        Router::new()
            .route(&format!("{}/monitor/ping", prefix), get(ping))
            .route(&format!("{}/incoming", prefix), post(incoming))
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

// ---------- route implementations below

/// Respond to ping. Useful for monitoring.
pub async fn ping(State(loudie): State<Arc<LoudbotFace>>) -> String {
    if let Some(yell) = loudie.brain.random_yell().await {
        yell
    } else {
        "failed to find yell".to_string()
    }
}

/// Handle an incoming post from Slack.
#[axum_macros::debug_handler]
async fn incoming(State(loudie): State<Arc<LoudbotFace>>, Json(incoming): Json<IncomingEvent>) -> Response {
    // if the token doesn't match, yell and bail
    if incoming.token != loudie.verification {
        return (StatusCode::BAD_REQUEST, "invalid payload".to_string()).into_response();
    }

    let Some(ref msgtype) = incoming.message_type else {
        dbg!(&incoming);
        return (StatusCode::IM_A_TEAPOT, "I'm a teapot".to_string()).into_response();
    };

    match msgtype.as_str() {
        "url_verification" => {
            let challenger = incoming.rest["challenge"].as_str().unwrap_or_default().to_string();
            let retort = serde_json::json!({
                "challenge": challenger,
            });
            (StatusCode::OK, retort.to_string()).into_response()
        }
        "event_callback" => {
            if let Some(event) = incoming.event {
                match loudie.handle_message(&event).await {
                    Ok(_) => log::debug!("handled callback successfully"),
                    Err(e) => log::warn!("error handling callback: {:?}", e),
                }
            } else {
                log::warn!("incoming post did not have a valid structure {:?}", incoming);
            }
            // respond with 200 OK no matter what (we should do this immediately, but we
            // can't)
            StatusCode::OK.into_response()
        }
        _ => {
            log::info!("unhandled type: {}", msgtype);
            StatusCode::OK.into_response()
        }
    }
}
