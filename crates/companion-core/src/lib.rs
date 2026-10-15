//! Downlink's pure engine: no I/O except reading and writing the config file.

pub mod config;
mod engine;
mod layer_stack;
mod tray_view;

pub use config::{Appearance, Config};
pub use engine::{
    Connection, Effect, Engine, FIRST_LAYOUT_RETRY, Input, LayoutState, MAX_LAYOUT_RETRY,
};
pub use layer_stack::LayerStack;
pub use tray_view::TrayView;
