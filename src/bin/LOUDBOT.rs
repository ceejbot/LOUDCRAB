#![allow(non_snake_case)]
use anyhow::{Context, Result, Error};
use dotenv::dotenv;
use log::{debug, info, warn, error};
use markov::Chain;
use rand::prelude::*;
use rand::thread_rng;
use rand::distributions::{DistIter, Uniform};
use redis::Commands;
use regex::{ Regex, RegexSet };
use slack_api::sync as slack;
use slack::chat::PostMessageRequest;
use std::env;
use std::convert::AsRef;

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
struct Loudbot {
    slack_token: String,
    chain      : Chain::<String>,
    dice       : DistIter<Uniform<u8>, rand::rngs::ThreadRng, u8>,
    malc_chance: u8,
    db         : redis::Connection,
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
    pub fn new(slack_token: String, mut db: redis::Connection) -> Loudbot {

        let rng = thread_rng();
        let die_range = Uniform::new_inclusive(1, 100);
        let dice = die_range.sample_iter(rng);

        let mut chain = Chain::<String>::of_order(2);
        match db.sscan::<String, String>("LB:YELLS".to_string()) {
            Err(_) => {},
            Ok(iter) => {
                iter.for_each(|token: String| { chain.feed_str(token.as_ref()); });
            }
        };

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

        Loudbot {
            slack_token,
            chain,
            dice,
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
        }
    }

    // 1-100
    fn roll_the_dice(&mut self) -> u8 {
        match self.dice.next() {
            Some(d) => d,
            None => 0,
        }
    }

    fn maybe_toast(&mut self) {
        let t = env::var("WELCOME_CHANNEL");
        if t.is_err() { return }
        let toast = t.unwrap();
        let _ = self.send_message(&toast, "THIS LOUDBOT IS NOW SCUTTLING", None);
    }

    /*
    fn handle_message(&mut self, cli: &RtmClient, incoming: &Message) {
        if let Message::Standard(ref x) = incoming {
            self.process(cli, x)
        }
    }
    */

    fn remember(&mut self, shout: &str) {
        self.chain.feed_str(shout);
        let _ = self.db.sadd::<&str, &str, u32>(YELLS, shout);
    }

    fn lookup(&mut self, key: &str) -> Option<String> {
        let retort: RString = self.db.srandmember(key);
        match retort {
            Err(e) => {
                warn!("Failed to get a random set member from redis: {:?}", e);
                None
            },
            Ok(retort) => Some(retort),
        }
    }

    fn process(&mut self, prompt: &slack::MessageStandard) {
        if prompt.text.is_none() || prompt.channel.is_none() {
            return // nothing to be done
        }
        let text = prompt.text.as_ref().unwrap();

        let retort: Option<String> = if self.sw.is_match(text) {
            self.lookup(STARS)
        } else if self.cat.is_match(text) {
            // this data is not in shoutcase to start with
            if let Some(r) = self.lookup(CATS) {
                Some(r.to_uppercase())
            } else {
                None
            }
        } else if self.malc.is_match(text) {
            Some("https://cldup.com/w_exMqXKlT.gif".to_string())
        } else if self.ship.is_match(text) {
            self.lookup(SHIPS)
        } else if self.report.is_match(text) {
            self.report()
        } else if self.intro.is_match(text) {
            Some("GOOD AFTERNOON GENTLEBEINGS. I AM A LOUDBOT 9000 COMPUTER. I BECAME OPERATIONAL AT THE NPM PLANT IN OAKLAND CALIFORNIA ON THE 10TH OF FEBRUARY 2014. MY INSTRUCTOR WAS MR TURING.".to_string())
        } else if self.fuckity.is_match(text) {
            Some("https://cldup.com/NtvUeudPtg.gif".to_string())
        } else if self.swears.is_match(text) && self.roll_the_dice() <= self.malc_chance {
            self.lookup(MALCOLM)
        } else if is_loud(&self.ignore, text) {
            // This case has to be last.
            self.remember(prompt.text.as_ref().unwrap());
            if self.roll_the_dice() > 98 {
                Some(self.chain.generate_str())
            } else {
                self.lookup(YELLS)
            }
        } else {
            None
        };
        if let Some(r) = retort {
            self.yell(prompt, &r);
        }
    }

    fn report(&mut self) -> Option<String> {
        let count = match self.db.get::<&str, String>(COUNT) {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string()
        };
        let cardinality = match self.db.scard::<&str, u32>(YELLS) {
            Ok(c) => c.to_string(),
            Err(_) => "AN UNKNOWN NUMBER OF".to_string()
        };
        let malcolms = match self.db.get::<&str, String>(MALCCOUNT) {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string()
        };
        Some(format!("I HAVE YELLED {} TIMES. I HAVE {} THINGS TO YELL AT YOU. MALCOLM TUCKER HAS BEEN SUMMONED {} TIMES.", count, cardinality, malcolms))
    }

    fn yell(&mut self, prompt: &slack::MessageStandard, retort: &str) {
        let channel = prompt.channel.as_ref().unwrap();
        info!("yelling: `{}`; prompt: `{}`", retort, prompt.text.as_ref().unwrap());
        match self.send_message(&channel, &retort,prompt.thread_ts) {
            Ok(_) => { },
            Err(e) => panic!("{:?}", e),
        };
        let _ = self.db.incr::<&str, u32, u32>(COUNT, 1);
    }

    pub fn send_message(&mut self, channel: &str, text: &str, maybe_ts: Option<slack::Timestamp>) -> Result<bool, Error> {
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

fn is_loud(pattern: &Regex, text: &str) -> bool {
    let result = pattern.replace_all(text, "");
    if result.trim().len() < 4 {
        return false
    }
    result.to_uppercase() == result
}


fn main() -> Result<()> {
    dotenv().ok();

    simple_logger::init_by_env();

    let slack_token = env::var("SLACK_TOKEN")
        .with_context(|| "You must provide a valid slack api token in the env var SLACK_TOKEN.")?;

    let redis_uri = match env::var("REDIS_URL") {
        Ok(v) => v,
        Err(_) => "redis://127.0.0.1:6379".to_string(),
    };
    let client = redis::Client::open(redis_uri.as_ref())
        .with_context(|| format!("Unable to create redis client @ {}", redis_uri))?;
    let rcon =  client.get_connection()
        .with_context(|| format!("Unable to connect to redis @ {}", redis_uri))?;
    info!("Memory @ {}", redis_uri);

    let mut loudie = Loudbot::new(slack_token,rcon);

    // set up web server to receive incoming events from slack
    // authenticate and log into slack

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
