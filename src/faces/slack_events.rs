//! Integration with Slack's latest as of Dec 2025 API,
//! via the slack-morphism crate.

use crate::IsLoudbotIntegration;
use crate::Loudbot;

use async_trait::async_trait;
use slack_morphism::SlackClient;

/*
SLACK_CLIENT_ID, SLACK_CLIENT_SECRET,
SLACK_BOT_SCOPE, SLACK_REDIRECT_HOST - for OAuth routes for Events API example
SLACK_SIGNING_SECRET
*/

#[derive(Debug, Clone)]
pub struct CarcinoMorphic {
    brain: Loudbot,
    client: SlackClient<SCHC>,
}

#[async_trait]
impl IsLoudbotIntegration for CarcinoMorphic {
    type Msg = Something;

    fn create(brain: Loudbot) -> Self {
        // let client =
        Self { brain, client }
    }

    async fn verify_request() -> anyhow::Result<bool> {
        todo!()
    }

    async fn maybe_toast(&self) -> anyhow::Result<bool> {
        todo!()
    }

    async fn handle_message(&self, incoming: &Self::Msg) -> anyhow::Result<bool> {
        todo!()
    }

    async fn yell(&self, prompt: &Self::Msg, retort: &str) -> anyhow::Result<bool> {
        todo!()
    }
}
