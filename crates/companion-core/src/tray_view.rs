use crate::engine::{Connection, Engine};

/// What the menu-bar item shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayView {
    /// Drives the icon: normal when connected, dimmed otherwise.
    pub connected: bool,
    pub status: String,
    /// Empty when not connected.
    pub detail: String,
    pub paused: bool,
}

impl Engine {
    pub fn tray_view(&self, source_name: impl Fn(&str) -> Option<String>) -> TrayView {
        let (connected, status) = match self.connection() {
            Connection::Connected { model, .. } => (true, format!("● {} connected", model.name())),
            Connection::Searching => (false, "Waiting for Voyager…".to_owned()),
            Connection::Unsupported { reason } => (false, reason.clone()),
            Connection::OpenFailed { reason } => (false, format!("Can't open keyboard: {reason}")),
        };
        let detail = if connected {
            let layer = self
                .active_layer()
                .map(|l| self.layer_title(l))
                .unwrap_or_else(|| "—".into());
            let input = self
                .current_source()
                .map(|id| source_name(id).unwrap_or_else(|| id.to_owned()))
                .unwrap_or_else(|| "—".into());
            format!("Layer: {layer} · Input: {input}")
        } else {
            String::new()
        };
        TrayView {
            connected,
            status,
            detail,
            paused: self.paused(),
        }
    }
}
