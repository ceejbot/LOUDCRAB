#![allow(non_snake_case)]

pub mod brain;
use axum::async_trait;
pub use brain::*;

use slack_api as slack;

// has a loudbot brain
// brain has a detector & a redis
// main makes a face and puts a brain behind it
// main calls 'start listening' on the face and lets it rip

#[async_trait]
trait LoudbotFace {
    // start listening
    // handle incoming
    //     if the message is relevant, hand it to loudbot
    //     if loudbot responds with a retort, post it
    // post message (given some kind of abstraction, post a message)
    async fn send_message(
        &self,
        channel: &str,
        text: &str,
        maybe_ts: Option<slack::Timestamp>,
    ) -> anyhow::Result<bool>;
    async fn handle_message(&self, incoming: slack::Message) -> anyhow::Result<bool>;
}
