//! THE SHOUTING ENGINE. This module glues the bot's memory (redis) to
//! the logic that selects retorts if appropriate. It is expected to be
//! consumed by a front end, such as a Slack bot client.
use anyhow::{Context, Result};
use async_once_cell::OnceCell;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use regex::Regex;

use std::convert::AsRef;

type RString = std::result::Result<String, redis::RedisError>;

use crate::triggers::*;

/// Our lazily-initialized redis connection.
static REDIS: OnceCell<MultiplexedConnection> = async_once_cell::OnceCell::new();
/// Redis key for count of times yelled
pub const COUNT: &str = "LB:COUNT";
/// The Redis key for the yell set.
pub const YELLS: &str = "LB:YELLS";

/// The LOUDBOT struct (sadly not shoutcased) is our app state.
///
/// This structure holds the slack response information as well as the redis
/// connection: anything we want to live through the whole process.
#[derive(Clone)]
pub struct Loudbot {
    /// our redis client
    client: redis::Client,
    /// Characters that should be stripped from a message before processing.
    ignore: Regex,
    /// Fun triggers to respond to
    triggers: Vec<Trigger>,
    /// The fearsome Malcolm Tucker
    malcolm: MalcolmSpecials,
    /// Are we asking for a self introduction?
    intro: Regex,
    /// Are we asking for a LOUDBOT self-report?
    report: Regex,
}

impl Loudbot {
    pub fn new(redis_uri: String, malc_chance: u8) -> Result<Loudbot, anyhow::Error> {
        let client = redis::Client::open(redis_uri.as_ref())
            .with_context(|| format!("Unable to create redis client @ {}", redis_uri))?;

        // More refactoring needed, but this is a step forward.
        let cats = Trigger::new(
            "CATS",
            Regex::new("(?i)CAT +FACT").unwrap(),
            include_str!("data/CATS").split('\n').map(|x| x.to_string()).collect(),
            100,
        );
        let stars = Trigger::new(
            "STARS",
            Regex::new(SW).unwrap(),
            include_str!("data/STAR_FIGHTING")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
            100,
        );
        let ships = Trigger::new(
            "SHIPS",
            Regex::new(r"(?i)\b(SHIP ?NAME|CULTURE +SHIP)\b").unwrap(),
            include_str!("data/SHIPS").split('\n').map(|x| x.to_string()).collect(),
            100,
        );
        let strategies = Trigger::new(
            "STRATEGIES",
            Regex::new(r"(?i)\bOBLIQUE +STRATEG(Y|IES)\b").unwrap(),
            include_str!("data/STRATEGIES")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
            100,
        );

        let malcolm = Trigger::new(
            "MALC",
            Regex::new(r"(?i)(.*FUCK.*|\bCUNT\b|\bTWAT\b|\bOMNISHAMBLES\b)").unwrap(),
            include_str!("data/MALCOLM")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
            malc_chance,
        );

        let triggers = vec![cats, stars, ships, strategies, malcolm];

        let malcolm = MalcolmSpecials::new(malc_chance);

        Ok(Loudbot {
            client,
            triggers,
            malcolm,
            intro: Regex::new("(?i)LOUDBOT +INTRODUCE +YOURSELF").unwrap(),
            report: Regex::new("(?i)LOUDBOT +REPORT").unwrap(),
            ignore: Regex::new(IGNORE).unwrap(),
        })
    }

    /// Fetch our persistent redis connection
    async fn redis(&self) -> &MultiplexedConnection {
        REDIS
            .get_or_init(async {
                match self.client.get_multiplexed_async_std_connection().await {
                    Ok(db) => db,
                    Err(e) => panic!("{}", e),
                }
            })
            .await
    }

    pub async fn random_yell(&self) -> Option<String> {
        self.select(YELLS).await
    }

    /// This is special because all existing loudbots count yells specially. sadly.
    pub async fn increment_yells(&self) {
        self.increment(COUNT).await;
    }

    // TODO: collapse process() and classify(); the retort type needs to go away

    /// Examine a text string and decide if we want to retort. We handle all our own
    /// internal storage concerns here, and respond to the interface layer with
    /// either response text or None.
    pub async fn process(&self, text: &str) -> Option<String> {
        match self.classify(text) {
            Retort::None => None,
            Retort::Canned(r) => Some(r),
            Retort::Report => self.report().await,
            Retort::Remember(set) => {
                // In this order so we don't yell the input back.
                let yell = self.select(&set).await;
                self.remember(&set, text).await;
                yell
            }
            Retort::Trigger { retort, set } => {
                // Every named trigger has a corresponding counter.
                let counter = format!("{set}_COUNT");
                self.increment(&counter).await;
                Some(retort)
            }
        }
    }

    /// Examine an incoming text message and decide if we want to shout at it.
    ///
    /// First we decide if the message qualifies for any of our special responses, using
    /// the extremely high-tech regex approach. Then we decide if the message is a shout
    /// and if so, we shout back.
    pub fn classify(&self, text: &str) -> Retort {
        if let Some(response) = self.triggers.iter().find_map(|t| t.maybe_respond(text)) {
            response
        } else if let Some(response) = self.malcolm.maybe_respond(text) {
            response
        } else if self.report.is_match(text) {
            Retort::Report // requires async work
        } else if self.intro.is_match(text) {
            Retort::Canned("GOOD AFTERNOON GENTLEBEINGS. I AM A LOUDBOT 9000 COMPUTER. I BECAME OPERATIONAL AT THE NPM PLANT IN OAKLAND CALIFORNIA ON THE 10TH OF FEBRUARY 2014. MY INSTRUCTOR WAS MR TURING.".to_string())
        } else if self.is_loud(text) {
            // This case has to be last.
            Retort::Remember(YELLS.to_string())
        } else {
            Retort::None
        }
    }

    /// Increment the named counter, ignoring errors because this is a nice-to-have not a requirement.
    async fn increment(&self, counter: &str) {
        let mut r = self.redis().await.clone();
        let _ = r.incr::<&str, u32, u32>(counter, 1_u32).await;
    }

    /// LOUDBOT REMEMBERS WHAT YOU SHOUT.
    async fn remember(&self, key: &str, shout: &str) {
        let mut r = self.redis().await.clone();
        let _ = r.sadd::<&str, &str, u32>(key, shout).await;
    }

    /// Select a random message from the named message set. This is used only for the core shouts.
    pub async fn select(&self, key: &str) -> Option<String> {
        let mut r = self.redis().await.clone();
        let retort: RString = r.srandmember(key).await;
        match retort {
            Err(e) => {
                log::warn!("Failed to get a random set member from redis: {:?}", e);
                None
            }
            Ok(retort) => Some(retort.to_uppercase()),
        }
    }

    /// Is the input LOUD or not?
    ///
    /// Believe it or not, this is the hardest job a LOUDBOT has. You don't want it
    /// shouting out of turn or in response to slack user mentions or html.
    pub fn is_loud(&self, text: &str) -> bool {
        let result = self.ignore.replace_all(text, "");
        if result.trim().len() < 4 {
            return false;
        }
        result.to_uppercase() == result
    }

    async fn trigger_report(mut r: MultiplexedConnection, t: &Trigger) -> String {
        let key = format!("LB:{}_COUNT", t.set());
        let count = match r.get::<&str, String>(&key).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        format!("{count} {} SHOUTS", t.set())
    }

    /// Respond to the `report` command. This is the only remaining place
    /// that needs specific redis key strings.
    async fn report(&self) -> Option<String> {
        let mut r = self.redis().await.clone();

        let count = match r.get::<&str, String>(COUNT).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let cardinality = match r.scard::<&str, u32>(YELLS).await {
            Ok(c) => c.to_string(),
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let mut lines = futures::future::join_all(self.triggers.iter().map(|t| {
            let tmp = r.clone();
            async move { Loudbot::trigger_report(tmp, t).await }
        }))
        .await;

        let malcolms = match r.get::<&str, String>("LB:MALC_COUNT").await {
            Ok(c) => c,
            Err(_) => "ZERO".to_string(),
        };
        lines.push(format!("MALCOLM TUCKER HAS BEEN SUMMONED {malcolms} TIMES."));
        let more = lines.join(" ");

        let version = env!("CARGO_PKG_VERSION");
        Some(format!("I AM RUNNING LOUDOS VERSION {version}. I HAVE YELLED {count} TIMES. I HAVE {cardinality} THINGS TO YELL AT YOU. {more}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_loud_works() {
        let loudie = Loudbot::new("redis://127.0.0.1".to_string(), 0).expect("could not construct a loudbot");
        assert!(loudie.is_loud("THIS IS LOUD"));
        assert!(loudie.is_loud("THIS IS LOUD."));
        assert!(loudie.is_loud("YOU ARE EXTREMELY SILLY <@U123> OH YEAH"));
        assert!(loudie.is_loud("SHOUTING :fish: MOAR"));

        assert!(!loudie.is_loud("This is not loud"));
        assert!(!loudie.is_loud("12345"));
        assert!(!loudie.is_loud("800-555-1212"));
        assert!(!loudie.is_loud("FU!!!!!"));
        assert!(!loudie.is_loud("<@U123>"));
        assert!(!loudie.is_loud("ABC"));
        assert!(!loudie.is_loud("1234-1249384 <@U123> 912302"));
        assert!(!loudie.is_loud("<@U123> ABC"));
        assert!(!loudie.is_loud(":emoji1: :emoji2:"));
        assert!(!loudie.is_loud("not shouting :emoji:"));
    }

    #[test]
    fn scunthorpe_problem() {
        let loudie = Loudbot::new("redis://127.0.0.1".to_string(), 100).expect("could not construct a loudbot");
        match loudie.classify("FUCK YOU") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "MALC".to_string())
            }
            _ => unreachable!("we should have invoked the Tucker"),
        }
        assert!(
            matches!(
                loudie.classify("you are a complete omnishambles"),
                Retort::Trigger { retort: _, set: _ }
            ),
            "basic swearing should be detected"
        );

        assert!(
            matches!(loudie.classify("cunt"), Retort::Trigger { retort: _, set: _ }),
            "extremely bad word should be matched"
        );

        assert!(
            matches!(loudie.classify("scunthorpe"), Retort::None),
            "we do not have the Scunthorpe problem"
        );

        assert!(matches!(
            loudie.classify("fuckity bye"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(
            matches!(loudie.classify("Malcolm Tucker"), Retort::None),
            "One invocation of the dread Malcolm is not enough"
        );
        assert!(
            matches!(
                loudie.classify("Malcolm Tucker Malcolm Tucker"),
                Retort::Trigger { retort: _, set: _ }
            ),
            "Two invocations of Malcolm summons him"
        );
    }

    #[test]
    fn malcolm_can_be_disabled() {
        let loudie = Loudbot::new("redis://127.0.0.1".to_string(), 0).expect("could not construct a loudbot");
        assert!(
            matches!(loudie.classify("fuck you"), Retort::None),
            "Malcolm is disabled at 0"
        );
        assert!(
            matches!(loudie.classify("fuckity bye"), Retort::None),
            "Malcolm is disabled at 0"
        );
        assert!(
            matches!(loudie.classify("Malcolm Tucker Malcolm Tucker"), Retort::None),
            "Malcolm is disabled at 0"
        );
    }

    #[test]
    fn we_get_cat_facts() {
        let loudie = Loudbot::new("redis://127.0.0.1".to_string(), 0).expect("could not construct a loudbot");
        match loudie.classify("cat  fact") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "CATS".to_string())
            }
            _ => unreachable!("we should have matched a trigger!"),
        }
        assert!(matches!(
            loudie.classify("cat fact"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(matches!(
            loudie.classify("cat    fact"),
            Retort::Trigger { retort: _, set: _ }
        ));
    }

    #[test]
    fn strategies_are_oblique() {
        let loudie = Loudbot::new("redis://127.0.0.1".to_string(), 0).expect("could not construct a loudbot");
        match loudie.classify("oblique strategy") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "STRATEGIES".to_string())
            }
            _ => unreachable!("we should have matched a trigger!"),
        }
        assert!(matches!(
            loudie.classify("oblique   strategy"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(matches!(
            loudie.classify("oblique  strategies"),
            Retort::Trigger { retort: _, set: _ }
        ));
    }

    #[test]
    fn we_have_no_gravitas() {
        let loudie = Loudbot::new("redis://127.0.0.1".to_string(), 0).expect("could not construct a loudbot");
        match loudie.classify("ship name") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "SHIPS".to_string())
            }
            _ => unreachable!("we should have matched a trigger!"),
        }
        assert!(matches!(
            loudie.classify("culture ship"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(matches!(
            loudie.classify("shipname"),
            Retort::Trigger { retort: _, set: _ }
        ));
    }
}
