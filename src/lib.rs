#![allow(non_snake_case)]

pub mod faces;
pub use faces::{IsLoudbotIntegration, LoudbotFace};

pub mod triggers;
pub use triggers::*;

pub mod loudbot;
pub use loudbot::Loudbot;
