//! THE SHOUTING ENGINE. This module glues the bot's memory (redis) to
//! the logic that selects retorts if appropriate. It is expected to be
//! consumed by a front end, such as a Slack bot client.
use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

use std::convert::AsRef;

type RString = std::result::Result<String, redis::RedisError>;

use crate::classifier::*;
use crate::{COUNT, YELLS};

/// The LOUDBOT struct (sadly not shoutcased) is our app state.
///
/// This structure holds the slack response information as well as the redis
/// connections: anything we want to live through the whole process.
#[derive(Clone)]
pub struct Loudbot {
    /// our redis connection
    db: MultiplexedConnection,
    /// the message classifier
    detector: Classifier,
}

impl Loudbot {
    pub async fn new(redis_uri: String, malc_chance: u8) -> Result<Loudbot, anyhow::Error> {
        let client = redis::Client::open(redis_uri.as_ref())
            .with_context(|| format!("Unable to create redis client @ {}", redis_uri))?;
        let db = client.get_multiplexed_async_std_connection().await?;

        let detector = Classifier::new(malc_chance);

        Ok(Loudbot { db, detector })
    }

    pub async fn random_yell(&self) -> Option<String> {
        self.select(YELLS).await
    }

    /// This is special because all existing loudbots count yells specially. sadly.
    pub async fn increment_yells(&self) {
        self.increment(COUNT).await;
    }

    /// Examine a text string and decide if we want to retort.
    pub async fn process(&self, text: &str) -> Option<String> {
        match self.detector.classify(text) {
            Retort::None => None,
            Retort::Canned(r) => Some(r),
            Retort::Random(set) => {
                // Every named set of responses has a corresponding counter.
                let counter = format!("LB:{set}_COUNT");
                self.increment(&counter).await;
                self.select(&format!("LB:{set}")).await
            }
            Retort::Report => self.report().await,
            Retort::Remember(set) => {
                // In this order so we don't yell the input back.
                let yell = self.select(&set).await;
                self.remember(&set, text).await;
                yell
            }
            Retort::Trigger { retort, set } => {
                // Every named set of responses has a corresponding counter.
                let counter = format!("{set}_COUNT");
                self.increment(&counter).await;
                Some(retort)
            }
        }
    }

    /// Increment the named counter, ignoring errors because this is a nice-to-have not a requirement.
    async fn increment(&self, counter: &str) {
        let mut r = self.db.clone();
        let _ = r.incr::<&str, u32, u32>(counter, 1_u32).await;
    }

    /// LOUDBOT REMEMBERS WHAT YOU SHOUT.
    async fn remember(&self, key: &str, shout: &str) {
        let mut r = self.db.clone();
        let _ = r.sadd::<&str, &str, u32>(key, shout).await;
    }

    /// Select a random message from the named message set.
    pub async fn select(&self, key: &str) -> Option<String> {
        let mut r = self.db.clone();
        let retort: RString = r.srandmember(key).await;
        match retort {
            Err(e) => {
                log::warn!("Failed to get a random set member from redis: {:?}", e);
                None
            }
            Ok(retort) => Some(retort.to_uppercase()),
        }
    }

    /// Respond to the `report` command. This is the only remaining place
    /// that needs specific redis key strings.
    async fn report(&self) -> Option<String> {
        let mut r = self.db.clone();

        let count = match r.get::<&str, String>(COUNT).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let cardinality = match r.scard::<&str, u32>(YELLS).await {
            Ok(c) => c.to_string(),
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let more = self.detector.report(r).await;

        let version = env!("CARGO_PKG_VERSION");
        Some(format!("I AM RUNNING LOUDOS VERSION {version}. I HAVE YELLED {count} TIMES. I HAVE {cardinality} THINGS TO YELL AT YOU. {more}"))
    }
}
