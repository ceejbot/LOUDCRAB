//! SHOUT, SHOUT, LET IT ALL OUT
#![allow(non_snake_case)]

use axum::{
    extract::Extension,
    routing::get,
    Router,
};
use dotenv::dotenv;
use slack::RtmClient;
use std::net::SocketAddr;
use std::sync::Arc;

use LOUDCRAB::{LoudbotRTM, Loudbot};

/// Respond to ping. Useful for monitoring.
async fn ping(Extension(loudie): Extension<Arc<Loudbot>>) -> String {
    if let Some(yell) = loudie.random_yell().await {
        yell
    } else {
        "failed to find yell".to_string()
    }
}

#[tokio::main]
async fn main() {
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
    match RtmClient::login_and_run(&slack_token, &mut face).await {
        Ok(_) => {}
        Err(err) => {
            log::error!("Caught error from rtm client; intentionally crashing");
            panic!("Error: {}", err);
        }
    }

    let app = Router::new()
        .route(&format!("{}/monitor/ping", prefix), get(ping))
        .layer(Extension(Arc::new(face)));

    let addr = format!("{}:{}", host, port);
    log::info!("LOUDBOT TUNED FOR SHOUTS COMING IN ON {}", &addr);

    let addr: SocketAddr = addr.parse().unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
