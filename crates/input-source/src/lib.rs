//! The OS keyboard input source (macOS: Text Input Sources; Linux: Hyprland's XKB layouts).

mod fake;
#[cfg(target_os = "linux")]
pub mod hyprland;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod unsupported;

pub use fake::FakeBackend;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSource {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InputSourceError {
    #[error("input source {0} is not installed or not enabled")]
    NotFound(String),
    #[error("{0}")]
    Platform(String),
    #[error("input-source switching is not supported on this platform yet")]
    Unsupported,
}

pub trait InputSourceBackend: Send {
    /// Enabled, selectable keyboard input sources, sorted by name.
    fn list(&self) -> Result<Vec<InputSource>, InputSourceError>;
    /// The ID of the current keyboard input source.
    fn current(&self) -> Result<String, InputSourceError>;
    fn select(&self, id: &str) -> Result<(), InputSourceError>;
}

/// The backend for the OS this binary was built for.
pub fn system_backend() -> Box<dyn InputSourceBackend> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::TisBackend)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(hyprland::HyprlandBackend)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Box::new(unsupported::UnsupportedBackend)
    }
}
