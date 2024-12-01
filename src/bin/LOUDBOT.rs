//! SHOUT, SHOUT, LET IT ALL OUT
//! This executable runs a slack loudie. It reads all config from its
//! environment, sourcing a `.env` file if one exists.
#![allow(non_snake_case)]
use std::sync::Arc;

use dotenvy::dotenv;
use LOUDCRAB::{IsLoudbotIntegration, Loudbot, LoudbotFace};

#[tokio::main]
async fn main() {
    dotenv().ok();
    simple_logger::init_with_env().ok();

    let redis_uri = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    log::info!("BRAIN @ {}", redis_uri);
    let prefix = std::env::var("ROUTE_PREFIX").unwrap_or_else(|_| "".to_string());
    let malc_chance: u8 = match std::env::var("TUCKER_CHANCE") {
        Ok(v) => match v.parse::<u8>() {
            Ok(x) => std::cmp::min(x, 100),
            Err(e) => {
                log::warn!("Failed to parse TUCKER_CHANCE as u8; falling back to 2%; {:?}", e);
                2
            }
        },
        Err(_) => 2,
    };

    let loudie = Loudbot::new(redis_uri, malc_chance).unwrap(); // intentional
    let face = LoudbotFace::create(loudie);
    let _ = face.maybe_toast().await; // ignoring errors

    let app = LoudbotFace::routes(prefix.as_str()).with_state(Arc::new(face));

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "6000".to_string());
    let bindstr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&bindstr)
        .await
        .unwrap_or_else(|_| panic!("COULD NOT BIND TO {}", bindstr));
    log::info!("LOUDBOT TUNED FOR SHOUTS COMING IN ON {}", &bindstr);
    axum::serve(listener, app).await.expect("LOUDBOT IS UNABLE TO SHOUT");
}
