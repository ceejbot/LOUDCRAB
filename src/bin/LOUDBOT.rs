//! SHOUT, SHOUT, LET IT ALL OUT
//! This executable runs a slack loudie. It reads all config from its
//! environment, sourcing a `.env` file if one exists.
#![allow(non_snake_case)]
use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use slack_api as slack;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use LOUDCRAB::{Loudbot, LoudbotSlack};

/// Respond to ping. Useful for monitoring.
async fn ping(Extension(loudie): Extension<Arc<Loudbot>>) -> String {
    if let Some(yell) = loudie.random_yell().await {
        yell
    } else {
        "failed to find yell".to_string()
    }
}

/// The parts of an incoming Slack webhook poast that we care about.
#[derive(Deserialize, Debug)]
struct IncomingEvent {
    /// Verification token, which must match what we expect.
    token: String,
    /// Type of the incoming message event.
    #[serde(rename = "type")]
    message_type: Option<String>,
    /// Full event payload.
    event: Option<slack::Message>,
    /// The remainder of the envelope, which is only needed sometimes.
    #[serde(flatten)]
    rest: HashMap<String, serde_json::Value>,
}

/// Handle an incoming post from Slack.
async fn incoming(
    Json(incoming): Json<IncomingEvent>,
    Extension(loudie): Extension<Arc<LoudbotSlack>>,
) -> (StatusCode, String) {
    // if the token doesn't match, yell and bail
    if incoming.token != loudie.verification {
        return (StatusCode::BAD_REQUEST, "invalid payload".to_string());
    }

    // This clone is to avoid a partial move of incoming so we can debug print later. I hate it.
    let res = if let Some(v) = incoming.message_type.clone() {
        if v == "url_verification" {
            let challenger = incoming.rest["challenge"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let retort = serde_json::json!({
                "challenge": challenger,
            });
            (StatusCode::OK, retort.to_string())
        } else if v == "event_callback" {
            if let Some(event) = incoming.event {
                match loudie.handle_message(event).await {
                    Ok(_) => log::debug!("handled callback successfully"),
                    Err(e) => log::warn!("error handling callback: {:?}", e),
                }
            } else {
                log::warn!(
                    "incoming post did not have a valid structure {:?}",
                    incoming
                );
            }
            // respond with 200 OK no matter what (we should do this immediately, but we can't)
            (StatusCode::OK, "OK".to_string())
        } else {
            log::info!("unhandled type: {}", v);
            (StatusCode::OK, "OK".to_string())
        }
    } else {
        dbg!(&incoming);
        (StatusCode::IM_A_TEAPOT, "I'm a teapot".to_string())
    };

    res
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    simple_logger::init_with_env().ok();

    let slack_token = std::env::var("SLACK_TOKEN")
        .expect("You must provide a valid slack api token in the env var SLACK_TOKEN.");
    let verification = std::env::var("VERIFICATION_TOKEN").expect(
        "You must provide your slack verification token in the env var VERIFICATION_TOKEN.",
    );

    let redis_uri =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    log::info!("BRAIN @ {}", redis_uri);
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "5000".to_string());
    let prefix = std::env::var("ROUTE_PREFIX").unwrap_or_else(|_| "".to_string());
    let malc_chance: u8 = match std::env::var("TUCKER_CHANCE") {
        Ok(v) => match v.parse::<u8>() {
            Ok(x) => std::cmp::min(x, 100),
            Err(e) => {
                log::warn!(
                    "Failed to parse TUCKER_CHANCE as u8; falling back to 2%; {:?}",
                    e
                );
                2
            }
        },
        Err(_) => 2,
    };

    let loudie = Loudbot::new(redis_uri, malc_chance).unwrap(); // intentional
    let face = LoudbotSlack::new(slack_token, verification, loudie);
    let _ = face.maybe_toast().await; // ignoring errors

    let app = Router::new()
        .route(&format!("{}/monitor/ping", prefix), get(ping))
        .route(&format!("{}/incoming", prefix), post(incoming))
        .layer(Extension(Arc::new(face)));

    let addr = format!("{}:{}", host, port);
    log::info!("LOUDBOT TUNED FOR SHOUTS COMING IN ON {}", &addr);

    let addr: SocketAddr = addr.parse().unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
