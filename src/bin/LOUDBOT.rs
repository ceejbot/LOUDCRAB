//! SHOUT, SHOUT, LET IT ALL OUT
#![allow(non_snake_case)]
use anyhow::{Context, Error, Result};
use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use rand::distributions::Uniform;
use rand::prelude::*;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use regex::{Regex, RegexSet};
use serde::Deserialize;
use slack::chat::PostMessageRequest;
use slack::{Message, MessageStandard};
use slack_api as slack;

use std::collections::HashMap;
use std::convert::AsRef;
use std::net::SocketAddr;
use std::sync::Arc;

type RString = std::result::Result<String, redis::RedisError>;

/// Characters to strip out before considering the loudness of the input. This pattern depends on the order of the chunks.
const IGNORE: &str = r":\w+:|<@\w+>|[\W\d[[:punct:]]]|s+";
/// The famous movie quote trigger pattern, extracted for testing.
const SW: &str = r"\b(?i)(LUKE +SKYWALKER|LEIA|SKYWALKER|ORGANA|TARKIN|LIGHTSABER|MILLENIUM +FALCON|DARTH +VADER|VADER|HAN +SOLO|OBIWAN|OBI-WAN|KENOBI|JABBA|CHEWIE|CHEWBACCA|TATOOINE|STAR +WARS?|DEATH +STAR|ALDERAAN|YAVIN|ENDOR)\b";

/// Roll a mythical d100.
fn roll_the_dice() -> u8 {
    let rng = thread_rng();
    let die_range = Uniform::new_inclusive(1, 100);
    let mut dice = die_range.sample_iter(rng);

    dice.next().unwrap_or(0)
}

/// Message retort types.
#[derive(Clone, Debug)]
enum Retort {
    /// No response wanted.
    None,
    /// Retort by yelling back a previous shout.
    Yell,
    /// Retort with a self-report
    Report,
    /// Retort with a random selection from the named message set.
    Random(String),
    /// Retort with a preset response.
    Canned(String),
}

/// Message classifier, extracted for ease of testing and to prevent having to recompile regexes.
#[derive(Clone, Debug)]
struct Detector {
    /// Characters that should be stripped from a message before processing.
    ignore: Regex,
    /// Are we asking for a cat fact?
    cat: Regex,
    /// Are we asking for a self introduction?
    intro: Regex,
    /// Are we asking for a LOUDBOT self-report?
    report: Regex,
    /// Culture ship names.
    ship: Regex,
    /// Key words from a famous movie.
    sw: Regex,
    /// A pattern detecting swearing (not safe for work, but y'all swear at work apparently.)
    swears: RegexSet,
    /// The percent chance that swearing will trigger a Malcolm Tucker gif.
    malc_chance: u8,
    /// A pattern detecting explicit invocation of Malcolm Tucker.
    malc: Regex,
    /// "Fuckity bye" gets a special from Malcolm Tucker.
    fuckity: Regex,
}

impl Detector {
    pub fn new(malc_chance: u8) -> Self {
        Detector {
            cat: Regex::new("(?i)CAT +FACT").unwrap(),
            fuckity: Regex::new("(?i)FUCKITY.?BYE").unwrap(),
            intro: Regex::new("(?i)LOUDBOT +INTRODUCE +YOURSELF").unwrap(),
            malc: Regex::new("(?i)MALCOLM +TUCKER +MALCOLM +TUCKER").unwrap(),
            malc_chance,
            report: Regex::new("(?i)LOUDBOT +REPORT").unwrap(),
            ship: Regex::new("(?i)SHIP ?NAME").unwrap(),
            ignore: Regex::new(IGNORE).unwrap(),
            sw: Regex::new(SW).unwrap(),
            swears: regex::RegexSet::new(&[
                r"(?i).*FUCK.*",
                r"(?i)(^|\W)CUNT(\W|$)",
                r"(?i)(^|\W)TWAT(\W|$)",
                r"(?i)(^|\W)OMNISHAMBLES(^|\W)",
            ])
            .unwrap(),
        }
    }

    /// Examine an incoming text message and decide if we want to shout at it.
    ///
    /// First we decide if the message qualifies for any of our special responses, using
    /// the extremely high-tech regex approach. Then we decide if the message is a shout
    /// and if so, we shout back.
    pub fn classify(&self, text: &str) -> Retort {
        if self.sw.is_match(text) {
            Retort::Random(STARS.to_string())
        } else if self.cat.is_match(text) {
            Retort::Random(CATS.to_string())
        } else if self.ship.is_match(text) {
            Retort::Random(SHIPS.to_string())
        } else if self.report.is_match(text) {
            Retort::Report
        } else if self.intro.is_match(text) {
            Retort::Canned("GOOD AFTERNOON GENTLEBEINGS. I AM A LOUDBOT 9000 COMPUTER. I BECAME OPERATIONAL AT THE NPM PLANT IN OAKLAND CALIFORNIA ON THE 10TH OF FEBRUARY 2014. MY INSTRUCTOR WAS MR TURING.".to_string())
        } else if self.malc_chance > 0 && self.malc.is_match(text) {
            Retort::Canned("https://cldup.com/w_exMqXKlT.gif".to_string())
        } else if self.malc_chance > 0 && self.fuckity.is_match(text) {
            Retort::Canned("https://cldup.com/NtvUeudPtg.gif".to_string())
        } else if self.deserves_malcolm(text) {
            Retort::Random(MALCOLM.to_string())
        } else if self.is_loud(text) {
            // This case has to be last.
            Retort::Yell
        } else {
            Retort::None
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

    /// Is this swearing? And if so, do we pass our dice roll?
    pub fn deserves_malcolm(&self, text: &str) -> bool {
        self.swears.is_match(text) && roll_the_dice() <= self.malc_chance
    }
}

// These are redit key strings.
/// The Redis key for the yell set.
const YELLS: &str = "LB:YELLS";
/// Redis key for the set of possible yells
const STARS: &str = "LB:SW";
/// Redis key for set of famous movie quotes
const SHIPS: &str = "LB:SHIPS";
/// Redis key for set of Culture ship names
const CATS: &str = "LB:CAT";
/// Redis key for set of cat facts
const COUNT: &str = "LB:COUNT";
/// Redis key for count of times yelled
const MALCOLM: &str = "LB:MALC";

/// The LOUDBOT struct (sadly not shoutcased) is our Tide app state.
///
/// This structure holds the slack response information as well as the redis
/// connections: anything we want to live through the whole process.
#[derive(Clone)]
struct Loudbot {
    /// the API token we must send to Slack
    slack_token: String,
    /// the verification token Slack must send to us
    verification: String,
    /// our redis connection
    db: MultiplexedConnection,
    /// the message classifier
    detector: Detector,
}

impl Loudbot {
    pub async fn new(
        slack_token: String,
        verification: String,
        redis_uri: String,
    ) -> Result<Loudbot, anyhow::Error> {
        let client = redis::Client::open(redis_uri.as_ref())
            .with_context(|| format!("Unable to create redis client @ {}", redis_uri))?;
        let db = client.get_multiplexed_async_std_connection().await?;

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
        let detector = Detector::new(malc_chance);

        Ok(Loudbot {
            slack_token,
            verification,
            db,
            detector,
        })
    }

    /// If we have a welcome channel, send a toast to it.
    async fn maybe_toast(&self) -> anyhow::Result<bool> {
        if let Ok(toast) = std::env::var("WELCOME_CHANNEL") {
            self.send_message(&toast, "THIS LOUDBOT IS NOW SCUTTLING", None)
                .await
        } else {
            Ok(false)
        }
    }

    /// Process an incoming message from slack and make decisions based on its envelope.
    async fn handle_message(&self, incoming: Message) -> anyhow::Result<bool> {
        match incoming {
            Message::BotMessage(ref _y) => {
                log::debug!("skipping bot message");
                Ok(false)
            }
            Message::Standard(ref x) => {
                if let Some(_bot_id) = &x.bot_id {
                    log::info!("skipping bot message");
                    Ok(false)
                } else {
                    self.process(x).await
                }
            }
            _ => {
                // we're just ignoring it
                Ok(false)
            }
        }
    }

    /// Examine Slack message content and decide if we want to retort.
    async fn process(&self, prompt: &MessageStandard) -> anyhow::Result<bool> {
        if prompt.text.is_none() || prompt.channel.is_none() {
            return Ok(false); // nothing to be done
        }

        let text = prompt.text.as_ref().unwrap(); // we know this is safe
        let retort = match self.detector.classify(text) {
            Retort::None => None,
            Retort::Canned(r) => Some(r),
            Retort::Random(set) => {
                // Every named set of responses has a corresponding counter.
                let counter = format!("{set}_COUNT");
                self.increment(&counter).await;
                self.select(&set).await
            }
            Retort::Report => self.report().await,
            Retort::Yell => {
                self.remember(prompt.text.as_ref().unwrap()).await;
                self.select(YELLS).await
            }
        };

        if let Some(r) = retort {
            self.yell(prompt, &r).await
        } else {
            Ok(false)
        }
    }

    /// Increment the named counter, ignoring errors because this is a nice-to-have not a requirement.
    async fn increment(&self, counter: &str) {
        let mut r = self.db.clone();
        let _ = r.incr::<&str, u32, u32>(counter, 1_u32).await;
    }

    /// LOUDBOT REMEMBERS WHAT YOU SHOUT.
    async fn remember(&self, shout: &str) {
        let mut r = self.db.clone();
        let _ = r.sadd::<&str, &str, u32>(YELLS, shout).await;
    }

    /// Select a random message from the named message set.
    async fn select(&self, key: &str) -> Option<String> {
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

    /// Respond to the `report` command.
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
        let key = format!("{MALCOLM}_COUNT");
        let malcolms = match r.get::<&str, String>(&key).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        let version = env!("CARGO_PKG_VERSION");
        Some(format!("I AM RUNNING LOUDOS VERSION {version}. I HAVE YELLED {count} TIMES. I HAVE {cardinality} THINGS TO YELL AT YOU. MALCOLM TUCKER HAS BEEN SUMMONED {malcolms} TIMES."))
    }

    /// Post a yell and record that we're doing so. Prefer this function to yell.
    async fn yell(&self, prompt: &slack::MessageStandard, retort: &str) -> anyhow::Result<bool> {
        let channel = prompt.channel.as_ref().unwrap();
        log::info!(
            "yelling: `{retort}`; prompt: `{}`' channel: `{channel}`",
            prompt.text.as_ref().unwrap()
        );
        let sent = self.send_message(channel, retort, prompt.thread_ts).await?;
        self.increment(COUNT).await;
        Ok(sent)
    }

    /// Internal implementation of sending a message to Slack.
    pub async fn send_message(
        &self,
        channel: &str,
        text: &str,
        maybe_ts: Option<slack::Timestamp>,
    ) -> Result<bool, Error> {
        let message = PostMessageRequest {
            channel,
            text,
            thread_ts: maybe_ts,
            unfurl_links: Some(true),
            link_names: Some(true),
            ..PostMessageRequest::default()
        };

        let client = slack::default_client()?;
        let response = slack::chat::post_message(&client, &self.slack_token, &message).await;
        match response {
            Err(e) => {
                log::error!("error trying to post message: {:?}", e);
                Err(anyhow::anyhow!(e))
            }
            Ok(_) => Ok(true),
        }
    }
}

/// Respond to ping. Useful for monitoring.
async fn ping(Extension(loudie): Extension<Arc<Loudbot>>) -> String {
    if let Some(yell) = loudie.select(YELLS).await {
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
    Extension(loudie): Extension<Arc<Loudbot>>,
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

    let loudie = Loudbot::new(slack_token, verification, redis_uri)
        .await
        .unwrap(); // intentional
    let _ = loudie.maybe_toast().await; // ignoring errors

    let app = Router::new()
        .route(&format!("{}/monitor/ping", prefix), get(ping))
        .route(&format!("{}/incoming", prefix), post(incoming))
        .layer(Extension(Arc::new(loudie)));

    let addr = format!("{}:{}", host, port);
    log::info!("LOUDBOT TUNED FOR SHOUTS COMING IN ON {}", &addr);

    let addr: SocketAddr = addr.parse().unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_loud_works() {
        let detector = Detector::new(0);
        assert!(detector.is_loud("THIS IS LOUD"));
        assert!(detector.is_loud("THIS IS LOUD."));
        assert!(detector.is_loud("YOU ARE EXTREMELY SILLY <@U123> OH YEAH"));
        assert!(detector.is_loud("SHOUTING :fish: MOAR"));

        assert!(!detector.is_loud("This is not loud"));
        assert!(!detector.is_loud("12345"));
        assert!(!detector.is_loud("800-555-1212"));
        assert!(!detector.is_loud("FU!!!!!"));
        assert!(!detector.is_loud("<@U123>"));
        assert!(!detector.is_loud("ABC"));
        assert!(!detector.is_loud("1234-1249384 <@U123> 912302"));
        assert!(!detector.is_loud("<@U123> ABC"));
        assert!(!detector.is_loud(":emoji1: :emoji2:"));
        assert!(!detector.is_loud("not shouting :emoji:"));
    }

    #[test]
    fn movie_easter_egg_works() {
        let patt = Regex::new(SW).unwrap();

        assert!(patt.is_match("chewbacca"));
        assert!(patt.is_match("Chewbacca"));
        assert!(patt.is_match("ChewIE"));
        assert!(!patt.is_match("luke"));
        assert!(patt.is_match("luke skywalker"));
        assert!(!patt.is_match("fluke"));
        assert!(!patt.is_match("vendor"));
        assert!(patt.is_match("third moon of Endor"));
    }

    #[test]
    fn scunthorpe_problem() {
        let detector = Detector::new(100);
        assert!(
            detector.deserves_malcolm("FUCK YOU"),
            "basic swearing should be detected"
        );
        assert!(
            !detector.deserves_malcolm("scunthorpe"),
            "we don't have the Scunthorpe problem"
        );
        assert!(matches!(
            detector.classify("fuckity bye"),
            Retort::Canned(_)
        ));
        assert!(
            matches!(detector.classify("Malcolm Tucker"), Retort::None),
            "One invocation of the dread Malcolm is not enough"
        );
        assert!(
            matches!(
                detector.classify("Malcolm Tucker Malcolm Tucker"),
                Retort::Canned(_)
            ),
            "Two invocations of Malcolm summons him"
        );
    }

    #[test]
    fn malcolm_can_be_disabled() {
        let detector = Detector::new(0);
        assert!(
            !detector.deserves_malcolm("FUCK YOU"),
            "basic swearing is ignored"
        );
    }
}
