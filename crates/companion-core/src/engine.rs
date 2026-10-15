//! The pure state machine at the heart of Downlink: inputs in, effects out.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use oryx_device::DeviceEvent;
use oryx_layout::{Layout, geometry};
use oryx_protocol::{LayoutIds, Model};

use crate::config::Config;
use crate::layer_stack::LayerStack;

/// How long to wait before retrying the first failed layout fetch (spec §6.2).
pub const FIRST_LAYOUT_RETRY: Duration = Duration::from_secs(30);
/// The retry interval doubles after each failure, up to this.
pub const MAX_LAYOUT_RETRY: Duration = Duration::from_secs(600);

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
    /// `Some(layer)` pins a preview; `None` (or the live layer) follows live again.
    PreviewLayer(Option<u8>),
    RefreshLayout,
    /// The scheduled layout retry is due.
    RetryLayout,
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
    /// Every active layer, rebuilt from the layer reports and key presses.
    stack: LayerStack,
    /// Last source *Downlink* selected (spec §6.4 rule 5).
    last_selected: Option<String>,
    paused: bool,
    layout: LayoutState,
    current_source: Option<String>,
    /// Oryx indices of keys currently held down.
    pressed: BTreeSet<usize>,
    preview: Option<u8>,
    /// The scheduled layout retry, if any: how long after the failure it is due.
    retry: Option<Duration>,
    /// The interval the next failed fetch schedules.
    backoff: Duration,
}

impl Engine {
    pub fn new(config: Config) -> Engine {
        Engine {
            config,
            connection: Connection::Searching,
            layer: None,
            stack: LayerStack::default(),
            last_selected: None,
            paused: false,
            layout: LayoutState::None,
            current_source: None,
            pressed: BTreeSet::new(),
            preview: None,
            retry: None,
            backoff: FIRST_LAYOUT_RETRY,
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

    pub fn pressed(&self) -> &BTreeSet<usize> {
        &self.pressed
    }

    /// The active layers beneath `layer`, highest first: where its transparent keys fall through.
    pub fn layers_below(&self, layer: u8) -> Vec<u8> {
        self.stack.below(layer)
    }

    /// The layer the layout window shows: the pinned preview, else the live layer.
    pub fn shown_layer(&self) -> Option<u8> {
        self.preview.or(self.layer)
    }

    /// When the failed layout fetch should be tried again; `None` when no retry is scheduled.
    pub fn layout_retry_interval(&self) -> Option<Duration> {
        self.retry
    }

    pub fn is_following(&self) -> bool {
        self.preview.is_none()
    }

    pub fn layer_title(&self, index: u8) -> String {
        if let LayoutState::Ready(layout) = &self.layout
            && let Some(layer) = layout.layers.iter().find(|layer| layer.index == index)
            && !layer.title.trim().is_empty()
        {
            return layer.title.trim().to_owned();
        }
        format!("Layer {index}")
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
            Input::PreviewLayer(layer) => {
                self.preview = match layer {
                    Some(layer) if Some(layer) == self.layer => None,
                    other => other,
                };
                Vec::new()
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
                    // A manual refresh replaces the scheduled retry; the backoff keeps growing
                    // if it fails too.
                    self.retry = None;
                    effects
                }
                _ => Vec::new(),
            },
            Input::RetryLayout => self.on_retry_layout(),
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
                self.stack.layer_reported(layer);
                if self.layer == Some(layer) {
                    return Vec::new();
                }
                self.layer = Some(layer);
                let mut effects = self.apply_layer_rule();
                effects.push(Effect::RefreshTray);
                effects
            }
            DeviceEvent::KeyDown(pos) => {
                let index = self.key_index(pos);
                let key = index.and_then(|index| self.key_on_stack(index)).cloned();
                self.stack.key_pressed(key.as_ref());
                if let Some(index) = index {
                    self.pressed.insert(index);
                }
                Vec::new()
            }
            DeviceEvent::KeyUp(pos) => {
                if let Some(index) = self.key_index(pos) {
                    self.pressed.remove(&index);
                }
                Vec::new()
            }
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

    /// What the key at Oryx `index` does on the current layer stack.
    fn key_on_stack(&self, index: usize) -> Option<&oryx_layout::Key> {
        let LayoutState::Ready(layout) = &self.layout else {
            return None;
        };
        let top = self.stack.top();
        let layers = std::iter::once(top).chain(self.stack.below(top));
        layout.resolve_key(layers, index).map(|(_, key)| key)
    }

    fn key_index(&self, pos: oryx_protocol::MatrixPos) -> Option<usize> {
        match &self.connection {
            Connection::Connected { model, .. } => geometry::oryx_index(*model, pos),
            _ => None,
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
        self.stack.reset();
        self.pressed.clear();
        self.reset_layout_retry();
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
        self.stack.reset();
        self.pressed.clear();
        self.reset_layout_retry();
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
        match result {
            Ok(layout) => {
                self.layout = LayoutState::Ready(layout);
                self.reset_layout_retry();
            }
            Err(error) => {
                self.layout = LayoutState::Unavailable(error);
                // Only a connected keyboard with known ids has anything to retry.
                if matches!(self.connection, Connection::Connected { ids: Some(_), .. }) {
                    self.retry = Some(self.backoff);
                    self.backoff = (self.backoff * 2).min(MAX_LAYOUT_RETRY);
                }
            }
        }
        vec![Effect::RefreshTray]
    }

    fn reset_layout_retry(&mut self) {
        self.retry = None;
        self.backoff = FIRST_LAYOUT_RETRY;
    }

    /// The retry timer fired: fetch again (cache first, then Oryx), but only if the failure it
    /// was scheduled for is still the current state.
    fn on_retry_layout(&mut self) -> Vec<Effect> {
        if self.retry.is_none() || !matches!(self.layout, LayoutState::Unavailable(_)) {
            return Vec::new();
        }
        let Connection::Connected {
            model,
            ids: Some(ids),
            ..
        } = &self.connection
        else {
            return Vec::new();
        };
        let effect = Effect::FetchLayout {
            model: *model,
            ids: ids.clone(),
            force: false,
        };
        self.retry = None;
        self.layout = LayoutState::Loading;
        vec![effect]
    }
}
