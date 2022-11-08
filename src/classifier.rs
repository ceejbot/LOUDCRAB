use rand::distributions::Uniform;
use rand::prelude::*;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use regex::Regex;

use crate::YELLS;

/// Characters to strip out before considering the loudness of the input. This pattern depends on the order of the chunks.
pub const IGNORE: &str = r":\w+:|<@\w+>|[\W\d[[:punct:]]]|s+";
/// The famous movie quote trigger pattern, extracted for testing.
pub const SW: &str = r"\b(?i)(LUKE +SKYWALKER|LEIA|SKYWALKER|ORGANA|TARKIN|LIGHTSABER|MILLENIUM +FALCON|DARTH +VADER|VADER|HAN +SOLO|OBIWAN|OBI-WAN|KENOBI|JABBA|CHEWIE|CHEWBACCA|TATOOINE|STAR +WARS?|DEATH +STAR|ALDERAAN|YAVIN|ENDOR)\b";

/// Roll a mythical d100.
fn roll_the_dice() -> u8 {
    let rng = thread_rng();
    let die_range = Uniform::new_inclusive(1, 100);
    let mut dice = die_range.sample_iter(rng);

    dice.next().unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct Trigger {
    set: String,
    pattern: Regex,
    data: Vec<String>,
}

impl Trigger {
    fn set(&self) -> &str {
        &self.set
    }

    fn matches(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }

    fn response(&self, _text: &str) -> Retort {
        match self.data.choose(&mut rand::thread_rng()) {
            Some(retort) => Retort::Trigger {
                retort: retort.to_string(),
                set: self.set().to_string(),
            },
            None => Retort::None,
        }
    }

    fn new(base: &str, pattern: Regex, data: Vec<String>) -> Self {
        let set = base.to_string();
        Self { set, pattern, data }
    }
}

/// Message retort types.
#[derive(Clone, Debug)]
pub enum Retort {
    /// No response wanted.
    None,
    /// Select a random item from this category and then remember the input.
    Remember(String),
    /// Retort with a self-report
    Report,
    /// Retort with a random selection from the named message set.
    Random(String),
    /// This is a built-in trigger; it includes the response.
    Trigger { retort: String, set: String },
    /// Retort with a preset response.
    Canned(String),
}

/// Message classifier, extracted for ease of testing and to prevent having to recompile regexes.
#[derive(Clone, Debug)]
pub struct Classifier {
    /// Characters that should be stripped from a message before processing.
    ignore: Regex,
    /// Fun triggers to respond to
    triggers: Vec<Trigger>,
    /// Are we asking for a self introduction?
    intro: Regex,
    /// Are we asking for a LOUDBOT self-report?
    report: Regex,
    /// The percent chance that swearing will trigger a Malcolm Tucker gif.
    malc_chance: u8,
    /// The fearsome Malcolm Tucker
    malcolm: Trigger,
    /// A pattern detecting explicit invocation of Malcolm Tucker.
    malc: Regex,
    /// "Fuckity bye" gets a special from Malcolm Tucker.
    fuckity: Regex,
}

impl Classifier {
    pub fn new(malc_chance: u8) -> Self {
        // More refactoring needed, but this is a step forward.
        let cats = Trigger::new(
            "CATS",
            Regex::new("(?i)CAT +FACT").unwrap(),
            include_str!("data/CATS")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
        );
        let stars = Trigger::new(
            "STARS",
            Regex::new(SW).unwrap(),
            include_str!("data/STAR_FIGHTING")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
        );
        let ships = Trigger::new(
            "SHIPS",
            Regex::new(r"(?i)\b(SHIP ?NAME|CULTURE +SHIP)\b").unwrap(),
            include_str!("data/SHIPS")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
        );
        let strategies = Trigger::new(
            "STRATEGIES",
            Regex::new(r"(?i)\bOBLIQUE +STRATEG(Y|IES)\b").unwrap(),
            include_str!("data/STRATEGIES")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
        );

        let triggers = vec![cats, stars, ships, strategies];

        let malcolm = Trigger::new(
            "MALC",
            Regex::new(r"(?i)(.*FUCK.*|\bCUNT\b|\bTWAT\b|\bOMNISHAMBLES\b)").unwrap(),
            include_str!("data/MALCOLM")
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
        );

        Classifier {
            triggers,
            malcolm,
            fuckity: Regex::new("(?i)FUCKITY.?BYE").unwrap(),
            intro: Regex::new("(?i)LOUDBOT +INTRODUCE +YOURSELF").unwrap(),
            malc: Regex::new("(?i)MALCOLM +TUCKER +MALCOLM +TUCKER").unwrap(),
            malc_chance,
            report: Regex::new("(?i)LOUDBOT +REPORT").unwrap(),
            ignore: Regex::new(IGNORE).unwrap(),
        }
    }

    /// Examine an incoming text message and decide if we want to shout at it.
    ///
    /// First we decide if the message qualifies for any of our special responses, using
    /// the extremely high-tech regex approach. Then we decide if the message is a shout
    /// and if so, we shout back.
    pub fn classify(&self, text: &str) -> Retort {
        if let Some(trigger) = self.triggers.iter().find(|t| t.matches(text)) {
            return trigger.response(text);
        }

        if self.report.is_match(text) {
            Retort::Report
        } else if self.intro.is_match(text) {
            Retort::Canned("GOOD AFTERNOON GENTLEBEINGS. I AM A LOUDBOT 9000 COMPUTER. I BECAME OPERATIONAL AT THE NPM PLANT IN OAKLAND CALIFORNIA ON THE 10TH OF FEBRUARY 2014. MY INSTRUCTOR WAS MR TURING.".to_string())
        } else if self.malc_chance > 0 && self.malc.is_match(text) {
            Retort::Canned("https://cldup.com/w_exMqXKlT.gif".to_string())
        } else if self.malc_chance > 0 && self.fuckity.is_match(text) {
            Retort::Canned("https://cldup.com/NtvUeudPtg.gif".to_string())
        } else if self.malc_chance > 0
            && self.malcolm.matches(text)
            && roll_the_dice() <= self.malc_chance
        {
            self.malcolm.response(text)
        } else if self.is_loud(text) {
            // This case has to be last.
            Retort::Remember(YELLS.to_string())
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

    async fn trigger_report(mut r: MultiplexedConnection, t: &Trigger) -> String {
        let key = format!("LB:{}_COUNT", t.set());
        let count = match r.get::<&str, String>(&key).await {
            Ok(c) => c,
            Err(_) => "AN UNKNOWN NUMBER OF".to_string(),
        };
        format!("{} HAS BEEN TRIGGERED {count} TIMES.", t.set())
    }

    pub async fn report(&self, db: MultiplexedConnection) -> String {
        let mut lines = futures::future::join_all(self.triggers.iter().map(|t| {
            let r = db.clone();
            async move {
                Classifier::trigger_report(r, t).await
            }
        }))
        .await;

        let mut r = db.clone();
        let malcolms = match r.get::<&str, String>("LB:MALC_COUNT").await {
            Ok(c) => c,
            Err(_) => "ZERO".to_string(),
        };
        lines.push(format!(
            "MALCOLM TUCKER HAS BEEN SUMMONED {malcolms} TIMES."
        ));
        lines.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_loud_works() {
        let detector = Classifier::new(0);
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
    fn scunthorpe_problem() {
        let classifier = Classifier::new(100);
        match classifier.classify("FUCK YOU") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "MALC".to_string())
            }
            _ => unreachable!("we should have invoked the Tucker"),
        }
        assert!(
            matches!(
                classifier.classify("you are a complete omnishambles"),
                Retort::Trigger { retort: _, set: _ }
            ),
            "basic swearing should be detected"
        );

        assert!(
            matches!(
                classifier.classify("cunt"),
                Retort::Trigger { retort: _, set: _ }
            ),
            "extremely bad word should be matched"
        );

        assert!(
            matches!(classifier.classify("scunthorpe"), Retort::None),
            "we do not have the Scunthorpe problem"
        );

        assert!(matches!(
            classifier.classify("fuckity bye"),
            Retort::Canned(_)
        ));
        assert!(
            matches!(classifier.classify("Malcolm Tucker"), Retort::None),
            "One invocation of the dread Malcolm is not enough"
        );
        assert!(
            matches!(
                classifier.classify("Malcolm Tucker Malcolm Tucker"),
                Retort::Canned(_)
            ),
            "Two invocations of Malcolm summons him"
        );
    }

    #[test]
    fn malcolm_can_be_disabled() {
        let classifier = Classifier::new(0);
        assert!(
            matches!(classifier.classify("fuck you"), Retort::None),
            "Malcolm is disabled at 0"
        );
        assert!(
            matches!(classifier.classify("fuckity bye"), Retort::None),
            "Malcolm is disabled at 0"
        );
        assert!(
            matches!(
                classifier.classify("Malcolm Tucker Malcolm Tucker"),
                Retort::None
            ),
            "Malcolm is disabled at 0"
        );
    }

    #[test]
    fn we_get_cat_facts() {
        let classifier = Classifier::new(0);
        match classifier.classify("cat  fact") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "CATS".to_string())
            }
            _ => unreachable!("we should have matched a trigger!"),
        }
        assert!(matches!(
            classifier.classify("cat fact"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(matches!(
            classifier.classify("cat    fact"),
            Retort::Trigger { retort: _, set: _ }
        ));
    }

    #[test]
    fn strategies_are_oblique() {
        let classifier = Classifier::new(0);
        match classifier.classify("oblique strategy") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "STRATEGIES".to_string())
            }
            _ => unreachable!("we should have matched a trigger!"),
        }
        assert!(matches!(
            classifier.classify("oblique   strategy"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(matches!(
            classifier.classify("oblique  strategies"),
            Retort::Trigger { retort: _, set: _ }
        ));
    }

    #[test]
    fn we_have_no_gravitas() {
        let classifier = Classifier::new(0);
        match classifier.classify("ship name") {
            Retort::Trigger { retort: _, set } => {
                assert_eq!(set, "SHIPS".to_string())
            }
            _ => unreachable!("we should have matched a trigger!"),
        }
        assert!(matches!(
            classifier.classify("culture ship"),
            Retort::Trigger { retort: _, set: _ }
        ));
        assert!(matches!(
            classifier.classify("shipname"),
            Retort::Trigger { retort: _, set: _ }
        ));
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
}
