//! Downlink's pure engine: no I/O except reading and writing the config file.

pub mod config;
mod engine;

pub use config::{Appearance, Config};
pub use engine::{Connection, Effect, Engine, Input, LayoutState};
