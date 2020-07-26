#![allow(non_snake_case)]
use anyhow::{Context, Result, Error};
use dotenv::dotenv;
use log::{debug, info, warn, error};
use markov::Chain;
use rand::prelude::*;
use rand::thread_rng;
use rand::distributions::Uniform;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use regex::{ Regex, RegexSet };
use slack_api::sync as slack;
use slack::chat::PostMessageRequest;
use std::env;
use std::convert::AsRef;

type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

type RString = std::result::Result<String, redis::RedisError>;

// This pattern depends on the order of the chunks.
const IGNORE: &str = r":\w+:|<@\w+>|[\W\d[[:punct:]]]|s+";
const SW: &str = r"\b(?i)(LUKE|LEIA|SKYWALKER|ORGANA|TARKIN|LIGHTSABER|ENDOR|MILLENIUM +FALCON|DARTH|VADER|HAN +SOLO|OBIWAN|OBI-WAN|KENOBI|CHEWIE|CHEWBACCA|TATOOINE|STAR +WARS?|DEATH +STAR)\b";

const YELLS    : &str = "LB:YELLS";
const STARS    : &str = "LB:SW";
const SHIPS    : &str = "LB:SHIPS";
const CATS     : &str = "LB:CAT";
const COUNT    : &str = "LB:COUNT";
const MALCOLM  : &str = "LB:MALC";
const MALCCOUNT: &str = "LB:MALCCOUNT";

// This holds everything we want to allocate once at startup, because
// what's the point of writing in Rust if we don't eke out RAW PERF?

#[derive(Clone)]
struct Loudbot {
    slack_token: String,
    // chain      : Chain::<String>,
    malc_chance: u8,
    db         : MultiplexedConnection,
    cat        : Regex,
    fuckity    : Regex,
    intro      : Regex,
    malc       : Regex,
    report     : Regex,
    ship       : Regex,
    ignore     : Regex,
    sw         : Regex,
    swears     : RegexSet,
}

impl Loudbot {
    pub async fn new(slack_token: String, redis_uri: String) -> Result<Loudbot, BoxedError> {

        let client = redis::Client::open(redis_uri.as_ref())
            .with_context(|| format!("Unable to create redis client @ {}", redis_uri)).unwrap();
        let db = client
            .get_multiplexed_async_std_connection().await?;

        /*
        let mut chain = Chain::<String>::of_order(2);
        match db.sscan::<String, String>("LB:YELLS".to_string()).await {
            Err(_) => {},
            Ok(iter) => {
                iter.for_each(|token: String| { chain.feed_str(token.as_ref()); });
            }
        };
        */

        let malc_chance: u8 = match env::var("TUCKER_CHANCE") {
            Ok(v) => {
                match v.parse::<u8>() {
                    Ok(x) => std::cmp::min(x, 100),
                    Err(e) => {
                        warn!("Failed to parse TUCKER_CHANCE as u8; falling back to 2%; {:?}", e);
                        2
                    },
                }
            },
            Err(_) => 2,
        };

        Ok(Loudbot {
            slack_token,
            // chain,
            malc_chance,
            db,
            cat       : Regex::new("(?i)CAT +FACT").unwrap(),
            fuckity   : Regex::new("(?i)FUCKITY.?BYE").unwrap(),
            intro     : Regex::new("(?i)LOUDBOT +INTRODUCE +YOURSELF").unwrap(),
            malc      : Regex::new("(?i)MALCOLM +TUCKER").unwrap(),
            report    : Regex::new("(?i)LOUDBOT +REPORT").unwrap(),
            ship      : Regex::new("(?i)SHIP ?NAME").unwrap(),
            ignore    : Regex::new(IGNORE).unwrap(),
            sw        : Regex::new("(?i)(LUKE|LEIA|SKYWALKER|ORGANA|TARKIN|LIGHTSABER|MILLENIUM +FALCON|DARTH|VADER|HAN +SOLO|OBIWAN|OBI-WAN|KENOBI|CHEWIE|CHEWBACCA|TATOOINE|STAR +WAR|DEATH +STAR)").unwrap(),
            swears    : regex::RegexSet::new(&[
                r"(?i).*FUCK.*",
                r"(?i)(^|\W)CUNT(\W|$)",
                r"(?i)(^|\W)TWAT(\W|$)",
                r"(?i)(^|\W)OMNISHAMBLES(^|\W)",
            ]).unwrap(),
        })
    }

    // 1-100
    fn roll_the_dice(& self) -> u8 {
        let rng = thread_rng();
        let die_range = Uniform::new_inclusive(1, 100);
        let mut dice = die_range.sample_iter(rng);

        match dice.next() {
            Some(d) => d,
            None => 0,
        }
    }

    async fn maybe_toast(& self) {
        let t = env::var("WELCOME_CHANNEL");
        if t.is_err() { return }
        let toast = t.unwrap();
        let _ = self.send_message(&toast, "THIS LOUDBOT IS NOW SCUTTLING", None);
    }

    /*
    fn handle_message(& self, cli: &RtmClient, incoming: &Message) {
        if let Message::Standard(ref x) = incoming {
            self.process(cli, x)
        }
    }
    */

    async fn remember(& self, shout: &str) {
        // self.chain.feed_str(shout);
        let mut r = self.db.clone();
        let _ = r.sadd::<&str, &str, u32>(YELLS, shout).await;
    }

    async fn lookup(& self, key: &str) -> Option<String> {
        let mut r = self.db.clone();
        let retort: RString = r.srandmember(key).await;
        match retort {
            Err(e) => {
                warn!("Failed to get a random set member from redis: {:?}", e);
                None
            },
            Ok(retort) => Some(retort),
        }
    }

    async fn process(& self, prompt: &slack::MessageStandard) {
        if prompt.text.is_none() || prompt.channel.is_none() {
            return // nothing to be done
        }
        let text = prompt.text.as_ref().unwrap();

        let retort: Option<String> = if self.sw.is_match(text) {
            self.lookup(STARS).await
        } else if self.cat.is_match(text) {
            // this data is not in shoutcase to start with
            if let Some(r) = self.lookup(CATS).await {
                Some(r.to_uppercase())
            } else {
                None
            }
        } else if self.malc.is_match(text) {
            Some("https://cldup.com/w_exMqXKlT.gif".to_string())
        } else if self.ship.is_match(text) {
            self.lookup(SHIPS).await
        } else if self.report.is_match(text) {
            self.report().await
        } else if self.intro.is_match(text) {
            Some("GOOD AFTERNOON GENTLEBEINGS. I AM A LOUDBOT 9000 COMPUTER. I BECAME OPERATIONAL AT THE NPM PLANT IN OAKLAND CALIFORNIA ON THE 10TH OF FEBRUARY 2014. MY INSTRUCTOR WAS MR TURING.".to_string())
        } else if self.fuckity.is_match(text) {
            Some("https://cldup.com/NtvUeudPtg.gif".to_string())
        } else if self.swears.is_match(text) && self.roll_the_dice() <= self.malc_chance {
            self.lookup(MALCOLM).await
        } else if is_loud(&self.ignore, text) {
            // This case has to be last.
            self.remember(prompt.text.as_ref().unwrap());
            if self.roll_the_dice() > 98 {
                self.lookup(YELLS).await
                // Some(self.chain.generate_str())
            } else {
                self.lookup(YELLS).await
            }
        } else {
            None
        };
        if let Some(r) = retort {
            self.yell(prompt, &r);
        }
    }

    async fn report(& self) -> Option<String> {
        let mut r = self.db.clone();

        let count = match r.get::<&str, String>(COUNT).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string()
        };
        let cardinality = match r.scard::<&str, u32>(YELLS).await {
            Ok(c) => c.to_string(),
            Err(_) => "AN UNKNOWN NUMBER OF".to_string()
        };
        let malcolms = match r.get::<&str, String>(MALCCOUNT).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string()
        };
        Some(format!("I HAVE YELLED {} TIMES. I HAVE {} THINGS TO YELL AT YOU. MALCOLM TUCKER HAS BEEN SUMMONED {} TIMES.", count, cardinality, malcolms))
    }

    async fn yell(& self, prompt: &slack::MessageStandard, retort: &str) {
        let channel = prompt.channel.as_ref().unwrap();
        info!("yelling: `{}`; prompt: `{}`", retort, prompt.text.as_ref().unwrap());
        match self.send_message(&channel, &retort,prompt.thread_ts) {
            Ok(_) => { },
            Err(e) => panic!("{:?}", e),
        };

        let mut r = self.db.clone();
        let _ = r.incr::<&str, u32, u32>(COUNT, 1).await;
    }

    pub fn send_message(& self, channel: &str, text: &str, maybe_ts: Option<slack::Timestamp>) -> Result<bool, Error> {
        let message = PostMessageRequest {
            channel,
            text,
            thread_ts: maybe_ts,
            unfurl_links: Some(true),
            link_names: Some(true),
            ..PostMessageRequest::default()
        };

        // this error we let bubble up
        let client = slack::default_client()?;
        let response = slack::chat::post_message(
            &client,
            &self.slack_token,
            &message
        );
        match response {
            Err(e) => {
                // this error we just log and continue from
                error!("error trying to post message: {:?}", e);
                Ok(false)
            },
            Ok(_) => {
                Ok(true)
            }
        }
    }
}

async fn incoming(req: tide::Request<Loudbot>) -> tide::Result<String> {
    let loudie = req.state();
    let y = loudie.lookup(YELLS).await;
    if let Some(yell) = y {
        loudie.send_message("#rubberduck", &yell, None);
        Ok(yell)
    } else {
        Ok("failed to find yell".to_string())
    }
}

fn is_loud(pattern: &Regex, text: &str) -> bool {
    let result = pattern.replace_all(text, "");
    if result.trim().len() < 4 {
        return false
    }
    result.to_uppercase() == result
}


fn main() -> Result<(), BoxedError> {
    dotenv().ok();

    simple_logger::init_by_env();

    let slack_token = env::var("SLACK_TOKEN")
        .with_context(|| "You must provide a valid slack api token in the env var SLACK_TOKEN.")?;

    let redis_uri = match env::var("REDIS_URL") {
        Ok(v) => v,
        Err(_) => "redis://127.0.0.1:6379".to_string(),
    };

    let host = env::var("HOST").ok().unwrap_or_else(|| "localhost".to_string());
    let port = env::var("PORT").ok().unwrap_or_else(|| "5000".to_string());

    // set up web server to receive incoming events from slack
    // authenticate and log into slack
    smol::run(async {
        info!("BRAIN @ {}", redis_uri);

        let mut loudie = Loudbot::new(slack_token,redis_uri).await.unwrap();
        loudie.maybe_toast().await;

        let mut app = tide::with_state(loudie);
        let addr = format!("{}:{}", host, port);
        info!("LOUDBOT TUNED FOR SHOUTS COMING IN ON {}", &addr);

        app.at("/incoming").get(incoming);
        app.listen(addr.clone()).await.unwrap();
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_loud_works() {
        let patt = Regex::new(IGNORE).unwrap();

        assert!(is_loud(&patt, "THIS IS LOUD"));
        assert!(is_loud(&patt, "THIS IS LOUD."));
        assert!(is_loud(&patt, "YOU ARE EXTREMELY SILLY <@U123> OH YEAH"));
        assert!(is_loud(&patt, "SHOUTING :fish: MOAR"));

        assert!(!is_loud(&patt, "This is not loud"));
        assert!(!is_loud(&patt, "12345"));
        assert!(!is_loud(&patt, "800-555-1212"));
        assert!(!is_loud(&patt, "FU!!!!!"));
        assert!(!is_loud(&patt, "<@U123>"));
        assert!(!is_loud(&patt, "ABC"));
        assert!(!is_loud(&patt, "1234-1249384 <@U123> 912302"));
        assert!(!is_loud(&patt, "<@U123> ABC"));
        assert!(!is_loud(&patt, ":emoji1: :emoji2:"));
        assert!(!is_loud(&patt, "not shouting :emoji:"));
    }

    #[test]
    fn movie_easter_egg_works() {
        let patt = Regex::new(SW).unwrap();

        assert!(patt.is_match("chewbacca"));
        assert!(patt.is_match("Chewbacca"));
        assert!(patt.is_match("ChewIE"));
        assert!(patt.is_match("luke"));
        assert!(!patt.is_match("fluke"));
        assert!(!patt.is_match("vendor"));
        assert!(patt.is_match("third moon of Endor"));
    }
}
