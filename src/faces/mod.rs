//! All integrations for loudbot and their common traits.

use async_trait::async_trait;

#[cfg(feature = "webhooks")]
#[path = "."]
mod chosen {
    #[path = "slack_webhooks.rs"]
    mod slack_webhooks;
    pub use slack_webhooks::LoudHooks as LoudbotFace;
}

#[cfg(feature = "events")]
mod chosen {
    mod slack_events;
    pub use slack_events::CarcinoMorphic as LoudbotFace;
}

pub use chosen::LoudbotFace;

#[async_trait]
/// Trait that any loudbot integration must implement.
pub trait IsLoudbotIntegration {
    /// The integration crate's own message type.
    type Msg;

    /// Make one of these.
    fn create(brain: crate::Loudbot) -> Self;

    /// Given data about an incoming request, verify that it came from the expected source..
    async fn verify_request() -> anyhow::Result<bool>;
    /// Perhaps make an announcement about arriving in the integration's channel/server/other.
    async fn maybe_toast(&self) -> anyhow::Result<bool>;
    /// Process an incoming message from the integration and decide how to react.
    async fn handle_message(&self, incoming: &Self::Msg) -> anyhow::Result<bool>;
    /// Given a message payload, yell a retort to it.
    async fn yell(&self, prompt: &Self::Msg, retort: &str) -> anyhow::Result<bool>;
}
