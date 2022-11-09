use rand::distributions::Uniform;
use rand::prelude::*;
use regex::Regex;

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

/// Message retort types.
#[derive(Clone, Debug)]
pub enum Retort {
    /// No response wanted.
    None,
    /// Select a random item from this category and then remember the input.
    Remember(String),
    /// Retort with a self-report
    Report,
    /// This is a built-in trigger; it includes the response.
    Trigger { retort: String, set: String },
    /// Retort with a preset response.
    Canned(String),
}

/// An easter egg trigger. These random response sets are built into LOUDBOT.
/// Each trigger can have a chance of being invoked, though this feature isn't
/// used by anything except the Malcolm Tucker swearing trigger.
#[derive(Debug, Clone)]
pub struct Trigger {
    set: String,
    pattern: Regex,
    data: Vec<String>,
    chance: u8,
}

impl Trigger {
    pub fn set(&self) -> &str {
        &self.set
    }

    pub fn maybe_respond(&self, text: &str) -> Option<Retort> {
        if self.chance == 0 || roll_the_dice() > self.chance {
            return None;
        }
        if !self.pattern.is_match(text) {
            return None;
        }

        self.data
            .choose(&mut rand::thread_rng())
            .map(|retort| Retort::Trigger {
                retort: retort.to_string(),
                set: self.set().to_string(),
            })
    }

    pub fn new(base: &str, pattern: Regex, data: Vec<String>, chance: u8) -> Self {
        let set = base.to_string();
        Self {
            set,
            pattern,
            data,
            chance,
        }
    }
}

// Note refactoring opportunity: this has the same API surface as the other triggers
// but takes a little more configuration. Also, the implementation of matches is different.
// I'd like to pull this into a trait when I can figure out how to store these in a vector
// somewhere. A vec of Box<dyn Trigger> is not sized, however, so I can't store them on the
// loudbot struct as that needs to go into a an arc. Maybe a once_cell static? They aren't mutable.
#[derive(Debug, Clone)]
pub struct MalcolmSpecials {
    chance: u8,
    set: String,
    fuckity: Regex,
    summon: Regex,
}

impl MalcolmSpecials {
    pub fn new(chance: u8) -> Self {
        Self {
            chance,
            set: "MALC".to_string(),
            fuckity: Regex::new("(?i)FUCKITY.?BYE").unwrap(),
            summon: Regex::new("(?i)MALCOLM +TUCKER +MALCOLM +TUCKER").unwrap(),
        }
    }

    fn set(&self) -> &str {
        &self.set
    }

    pub fn maybe_respond(&self, text: &str) -> Option<Retort> {
        if self.chance == 0 || roll_the_dice() > self.chance {
            None
        } else if self.fuckity.is_match(text) {
            Some(Retort::Trigger {
                retort: "https://cldup.com/NtvUeudPtg.gif".to_string(),
                set: self.set().to_string(),
            })
        } else if self.summon.is_match(text) {
            Some(Retort::Trigger {
                retort: "https://cldup.com/w_exMqXKlT.gif".to_string(),
                set: self.set().to_string(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
