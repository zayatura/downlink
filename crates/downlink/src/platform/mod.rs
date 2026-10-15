//! OS-specific glue. Everything else in Downlink is platform-neutral.

pub mod launch_agent;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    activate_app, hide_dock_icon, refresh_launch_at_login, set_launch_at_login, system_mode,
};

#[cfg(not(target_os = "macos"))]
mod other;
#[cfg(not(target_os = "macos"))]
pub use other::{
    activate_app, hide_dock_icon, refresh_launch_at_login, set_launch_at_login, system_mode,
};
