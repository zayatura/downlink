//! OS-specific glue. Everything else in Downlink is platform-neutral.

pub mod autostart;
pub mod launch_agent;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    activate_app, on_reopen, on_terminate, refresh_launch_at_login, set_launch_at_login,
    show_in_dock, system_mode, window_platform_specific,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{
    activate_app, on_reopen, on_terminate, refresh_launch_at_login, set_launch_at_login,
    show_in_dock, system_mode, window_platform_specific,
};
