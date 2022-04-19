use rand::distributions::Uniform;
use rand::prelude::*;
use regex::{Regex, RegexSet};

// These are redit key strings. This needs to be cleaned up.
// really these are names of easter eggs

/// The Redis key for the yell set.
pub const YELLS: &str = "LB:YELLS";
/// Redis key for the set of possible yells
pub const STARS: &str = "LB:SW";
/// Redis key for set of famous movie quotes
pub const SHIPS: &str = "LB:SHIPS";
/// Redis key for set of Culture ship names
pub const CATS: &str = "LB:CAT";
/// Redis key for set of cat facts
pub const COUNT: &str = "LB:COUNT";
/// Redis key for count of times yelled
pub const MALCOLM: &str = "LB:MALC";

/// Characters to strip out before considering the loudness of the input. This pattern depends on the order of the chunks.
const IGNORE: &str = r":\w+:|<@\w+>|[\W\d[[:punct:]]]|s+";
/// The famous movie quote trigger pattern, extracted for testing.
const SW: &str = r"\b(?i)(LUKE +SKYWALKER|LEIA|SKYWALKER|ORGANA|TARKIN|LIGHTSABER|MILLENIUM +FALCON|DARTH +VADER|VADER|HAN +SOLO|OBIWAN|OBI-WAN|KENOBI|JABBA|CHEWIE|CHEWBACCA|TATOOINE|STAR +WARS?|DEATH +STAR|ALDERAAN|YAVIN|ENDOR)\b";

/// Roll a mythical d100.
pub fn roll_the_dice() -> u8 {
    let rng = thread_rng();
    let die_range = Uniform::new_inclusive(1, 100);
    let mut dice = die_range.sample_iter(rng);

    dice.next().unwrap_or(0)
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
    /// Retort with a preset response.
    Canned(String),
}

/// Message classifier, extracted for ease of testing and to prevent having to recompile regexes.
#[derive(Clone, Debug)]
pub struct Classifier {
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

impl Classifier {
    pub fn new(malc_chance: u8) -> Self {
        Classifier {
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

    /// Is this swearing? And if so, do we pass our dice roll?
    pub fn deserves_malcolm(&self, text: &str) -> bool {
        self.swears.is_match(text) && roll_the_dice() <= self.malc_chance
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
        let detector = Classifier::new(100);
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
        let detector = Classifier::new(0);
        assert!(
            !detector.deserves_malcolm("FUCK YOU"),
            "basic swearing is ignored"
        );
        assert!(
            matches!(detector.classify("fuckity bye"), Retort::None),
            "Malcolm is disabled at 0"
        );
        assert!(
            matches!(
                detector.classify("Malcolm Tucker Malcolm Tucker"),
                Retort::None
            ),
            "Malcolm is disabled at 0"
        );
    }
}
