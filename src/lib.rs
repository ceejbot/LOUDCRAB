#![allow(non_snake_case)]

pub mod faces;
pub use faces::*;

pub mod classifier;
pub use classifier::*;

pub mod loudbot;
pub use loudbot::Loudbot;

// TODO refactor
// These magic constants are redis key strings but are really the
// names of easter eggs. The "LB:" prefix is predictable.

/// Redis key for a set of URLs for GIFs of Malcolm Tucker.
pub const MALCOLM: &str = "LB:MALC";

/// Redis key for count of times yelled
pub const COUNT: &str = "LB:COUNT";
/// The Redis key for the yell set.
pub const YELLS: &str = "LB:YELLS";
