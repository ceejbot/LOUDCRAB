//! SHOUT, SHOUT, LET IT ALL OUT
#![allow(non_snake_case)]

use anyhow::{Context, Result};
use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use slack::api::Timestamp;
use slack::{api, Error, Event, Message, RtmClient};

use LOUDCRAB::{LoudbotRTM, Loudbot};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    simple_logger::init_with_env().ok();

    let slack_token = std::env::var("SLACK_TOKEN")
        .expect("You must provide a valid slack api token in the env var SLACK_TOKEN.");

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

    let brain = Loudbot::new(redis_uri, malc_chance)
        .await
        .unwrap(); // intentional

    let mut face = LoudbotRTM::new(slack_token.clone(), brain);
    match RtmClient::login_and_run(&slack_token, &mut face) {
        Ok(_) => {}
        Err(err) => {
            log::error!("Caught error from rtm client; intentionally crashing");
            panic!("Error: {}", err);
        }
    }

    Ok(())
}
