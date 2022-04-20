use axum::async_trait;

// Let's start with slack-specific signatures for the moment...
use slack_api as slack;
use super::Loudbot;

mod slack_webhooks;
pub use slack_webhooks::LoudbotSlack;

// has a loudbot brain
// brain has a detector & a redis
// main makes a face and puts a brain behind it
// main calls 'start listening' on the face and lets it rip
#[async_trait]
trait _UnusedTrait {
    fn set_brain(brain: Loudbot);
    fn brain() -> &'static Loudbot;
    // start listening
    // handle incoming
    //     if the message is relevant, hand it to loudbot
    //     if loudbot responds with a retort, post it
    // post message (given some kind of abstraction, post a message)
    async fn verify_request() -> anyhow::Result<bool>;
    async fn handle_message(&self, incoming: slack::Message) -> anyhow::Result<bool>;
    async fn yell(&self, prompt: &slack::MessageStandard, retort: &str) -> anyhow::Result<bool>;
    async fn send_message(
        &self,
        channel: &str,
        text: &str,
        maybe_ts: Option<slack::Timestamp>,
    ) -> anyhow::Result<bool>;
}
