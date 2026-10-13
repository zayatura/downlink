//! Oryx layout data: parsing, Voyager geometry, legends, caching and fetching.

pub mod geometry;
mod model;

pub use model::{Action, Key, Layer, Layout, Modifiers, ParseError, parse_graphql};
