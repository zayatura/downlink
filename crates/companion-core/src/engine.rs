//! The pure state machine at the heart of Downlink: inputs in, effects out.

use std::sync::Arc;

use oryx_device::DeviceEvent;
use oryx_layout::Layout;
use oryx_protocol::{LayoutIds, Model};

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connection {
    Searching,
    Connected {
        model: Model,
        ids: Option<LayoutIds>,
        protocol: Option<u8>,
    },
    Unsupported {
        reason: String,
    },
    OpenFailed {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutState {
    None,
    Loading,
    Ready(Arc<Layout>),
    Unavailable(String),
}

#[derive(Debug, Clone)]
pub enum Input {
    Device(DeviceEvent),
    LayoutLoaded {
        ids: LayoutIds,
        result: Result<Arc<Layout>, String>,
    },
    CurrentSourceChanged(Option<String>),
    /// Selecting this source failed; allow retrying it on the next layer change.
    SelectFailed(String),
    ConfigChanged(Config),
    SetPaused(bool),
    RefreshLayout,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    SelectInputSource(String),
    /// Load the layout: from the cache first unless `force`, otherwise from Oryx.
    FetchLayout {
        model: Model,
        ids: LayoutIds,
        force: bool,
    },
    PersistConfig,
    RefreshTray,
    Exit,
}

pub struct Engine {
    config: Config,
    connection: Connection,
    layer: Option<u8>,
    /// Last source *Downlink* selected (spec §6.4 rule 5).
    last_selected: Option<String>,
    paused: bool,
    layout: LayoutState,
    current_source: Option<String>,
}

impl Engine {
    pub fn new(config: Config) -> Engine {
        Engine {
            config,
            connection: Connection::Searching,
            layer: None,
            last_selected: None,
            paused: false,
            layout: LayoutState::None,
            current_source: None,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn active_layer(&self) -> Option<u8> {
        self.layer
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn layout(&self) -> &LayoutState {
        &self.layout
    }

    pub fn current_source(&self) -> Option<&str> {
        self.current_source.as_deref()
    }

    pub fn handle(&mut self, input: Input) -> Vec<Effect> {
        match input {
            Input::Device(event) => self.on_device(event),
            Input::LayoutLoaded { ids, result } => self.on_layout_loaded(ids, result),
            Input::CurrentSourceChanged(id) => {
                if self.current_source == id {
                    return Vec::new();
                }
                self.current_source = id;
                vec![Effect::RefreshTray]
            }
            Input::SelectFailed(id) => {
                if self.last_selected.as_deref() == Some(id.as_str()) {
                    self.last_selected = None;
                }
                Vec::new()
            }
            Input::ConfigChanged(config) => {
                let mapping_changed = config.layers != self.config.layers;
                self.config = config;
                let mut effects = vec![Effect::PersistConfig, Effect::RefreshTray];
                if mapping_changed {
                    effects.extend(self.apply_layer_rule());
                }
                effects
            }
            Input::SetPaused(paused) => {
                self.paused = paused;
                vec![Effect::RefreshTray]
            }
            Input::RefreshLayout => match &self.connection {
                Connection::Connected {
                    model,
                    ids: Some(ids),
                    ..
                } => {
                    let effects = vec![Effect::FetchLayout {
                        model: *model,
                        ids: ids.clone(),
                        force: true,
                    }];
                    self.layout = LayoutState::Loading;
                    effects
                }
                _ => Vec::new(),
            },
            Input::Quit => {
                let mut effects = self.fallback_effect();
                effects.push(Effect::Exit);
                effects
            }
        }
    }

    fn on_device(&mut self, event: DeviceEvent) -> Vec<Effect> {
        match event {
            DeviceEvent::Connected {
                model,
                ids,
                protocol,
            } => self.on_connected(model, ids, protocol),
            DeviceEvent::Layer(layer) => {
                if self.layer == Some(layer) {
                    return Vec::new();
                }
                self.layer = Some(layer);
                let mut effects = self.apply_layer_rule();
                effects.push(Effect::RefreshTray);
                effects
            }
            // Pressed-key tracking is added in Task 17.
            DeviceEvent::KeyDown(_) | DeviceEvent::KeyUp(_) => Vec::new(),
            DeviceEvent::Disconnected { reason } => {
                tracing::info!(%reason, "keyboard disconnected");
                self.lose_connection(Connection::Searching)
            }
            DeviceEvent::Unsupported { reason } => {
                self.lose_connection(Connection::Unsupported { reason })
            }
            DeviceEvent::OpenFailed { reason } => {
                self.lose_connection(Connection::OpenFailed { reason })
            }
        }
    }

    fn on_connected(
        &mut self,
        model: Model,
        ids: Option<LayoutIds>,
        protocol: Option<u8>,
    ) -> Vec<Effect> {
        self.connection = Connection::Connected {
            model,
            ids: ids.clone(),
            protocol,
        };
        self.layer = None;
        let mut effects = vec![Effect::RefreshTray];
        match ids {
            Some(ids) => {
                let have_it = matches!(&self.layout, LayoutState::Ready(layout)
                    if layout.layout_id == ids.layout_id() && layout.revision_id == ids.revision_id());
                if !have_it {
                    self.layout = LayoutState::Loading;
                    effects.push(Effect::FetchLayout {
                        model,
                        ids,
                        force: false,
                    });
                }
            }
            None => {
                self.layout = LayoutState::Unavailable(
                    "the keyboard didn't report which layout it runs".into(),
                );
            }
        }
        effects
    }

    /// Spec §6.4 rule 2: the fallback fires only on a transition.
    fn lose_connection(&mut self, next: Connection) -> Vec<Effect> {
        let was_connected = matches!(self.connection, Connection::Connected { .. });
        let newly_unsupported = matches!(next, Connection::Unsupported { .. })
            && !matches!(self.connection, Connection::Unsupported { .. });
        let changed = self.connection != next;
        self.connection = next;
        self.layer = None;
        let mut effects = if changed {
            vec![Effect::RefreshTray]
        } else {
            Vec::new()
        };
        if was_connected || newly_unsupported {
            self.last_selected = None;
            effects.extend(self.fallback_effect());
        }
        effects
    }

    fn fallback_effect(&self) -> Vec<Effect> {
        if self.paused {
            return Vec::new();
        }
        self.config
            .fallback
            .clone()
            .map(Effect::SelectInputSource)
            .into_iter()
            .collect()
    }

    /// Spec §6.4 rule 1.
    fn apply_layer_rule(&mut self) -> Vec<Effect> {
        if self.paused {
            return Vec::new();
        }
        let Some(layer) = self.layer else {
            return Vec::new();
        };
        let Some(target) = self.config.layers.get(&layer) else {
            return Vec::new(); // transient layer
        };
        if self.last_selected.as_deref() == Some(target.as_str()) {
            return Vec::new();
        }
        self.last_selected = Some(target.clone());
        vec![Effect::SelectInputSource(target.clone())]
    }

    fn on_layout_loaded(
        &mut self,
        ids: LayoutIds,
        result: Result<Arc<Layout>, String>,
    ) -> Vec<Effect> {
        if !matches!(self.layout, LayoutState::Loading) {
            return Vec::new();
        }
        if let Connection::Connected {
            ids: Some(current), ..
        } = &self.connection
            && current != &ids
        {
            return Vec::new();
        }
        self.layout = match result {
            Ok(layout) => LayoutState::Ready(layout),
            Err(error) => LayoutState::Unavailable(error),
        };
        vec![Effect::RefreshTray]
    }
}
