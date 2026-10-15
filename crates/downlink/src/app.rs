//! Application state, messages and the effect executor.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use companion_core::{Appearance, Effect, Engine, Input, LayoutState, config};
use iced::theme::Mode;
use iced::{Element, Size, Subscription, Task, Theme, system, time, window};
use input_source::{InputSource, InputSourceBackend};
use oryx_layout::LayoutCache;
use oryx_layout::client::{self, HttpClient};
use oryx_protocol::{LayoutIds, Model};

use crate::device::{self, DeviceHandle, DeviceMessage};
use crate::layout_fetch::{FetchOutcome, resolve_fetch};
use crate::paths::Paths;
use crate::source_choice::SourceChoice;
use crate::tray::{self, AppMenu, MenuAction, Tray};
use crate::{layout_view, platform, settings_view, theme};

/// Set once the Quit path has run; `run()` uses it to decide on the exit safety net.
pub static QUIT_HANDLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Layout,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Init,
    Device(DeviceMessage),
    Menu(String),
    SystemTheme(Mode),
    WindowOpened(window::Id),
    WindowFocused(window::Id),
    WindowClosed(window::Id),
    PollSource,
    LayoutFetched {
        model: Model,
        ids: LayoutIds,
        from_cache: bool,
        result: Result<String, String>,
    },
    ChipPressed(u8),
    FollowLive,
    SetLayerSource(u8, SourceChoice),
    SetFallback(SourceChoice),
    SetAppearance(Appearance),
    SetLaunchAtLogin(bool),
    RefreshLayout,
    RetryLayout,
}

pub struct App {
    pub(crate) engine: Engine,
    pub(crate) paths: Paths,
    pub(crate) sources: Vec<InputSource>,
    pub(crate) system_mode: Mode,
    pub(crate) windows: BTreeMap<window::Id, WindowKind>,
    pub(crate) launch_error: Option<String>,
    pub(crate) cache: LayoutCache,
    backend: Box<dyn InputSourceBackend>,
    http: Option<HttpClient>,
    device: Option<DeviceHandle>,
    tray: Option<Tray>,
    app_menu: Option<AppMenu>,
    focused: Option<window::Id>,
    warned_missing: BTreeSet<String>,
}

impl App {
    pub fn boot(paths: Paths) -> (App, Task<Message>) {
        let config = crate::load_or_create_config(&paths.config_file);
        let http = match client::client_builder(env!("CARGO_PKG_VERSION")).build() {
            Ok(http) => Some(http),
            Err(error) => {
                tracing::error!(%error, "HTTP client unavailable; layouts load from the cache only");
                None
            }
        };
        let app = App {
            engine: Engine::new(config),
            cache: LayoutCache::new(paths.layout_cache_dir()),
            paths,
            sources: Vec::new(),
            system_mode: Mode::None,
            windows: BTreeMap::new(),
            launch_error: None,
            backend: input_source::system_backend(),
            http,
            device: None,
            tray: None,
            app_menu: None,
            focused: None,
            warned_missing: BTreeSet::new(),
        };
        (
            app,
            Task::batch([
                Task::done(Message::Init),
                system::theme().map(Message::SystemTheme),
            ]),
        )
    }

    pub fn title(&self, id: window::Id) -> String {
        match self.windows.get(&id) {
            Some(WindowKind::Settings) => "Downlink Settings".into(),
            _ => match self.engine.layout() {
                LayoutState::Ready(layout) => format!("Downlink — {}", layout.title),
                _ => "Downlink".into(),
            },
        }
    }

    pub fn theme(&self, _id: window::Id) -> Option<Theme> {
        Some(theme::iced_theme(self.is_dark()))
    }

    pub(crate) fn is_dark(&self) -> bool {
        theme::is_dark(self.engine.config().appearance, self.system_mode)
    }

    pub(crate) fn source_name(&self, id: &str) -> Option<String> {
        self.sources
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.name.clone())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let poll = if self.windows.is_empty() {
            Subscription::none()
        } else {
            time::every(Duration::from_secs(2)).map(|_| Message::PollSource)
        };
        // The first tick comes one interval after the failure, and a new interval restarts it.
        let retry = match self.engine.layout_retry_interval() {
            Some(interval) => time::every(interval).map(|_| Message::RetryLayout),
            None => Subscription::none(),
        };
        Subscription::batch([
            Subscription::run(device::events).map(Message::Device),
            Subscription::run(tray::menu_events).map(Message::Menu),
            system::theme_changes().map(Message::SystemTheme),
            window::close_events().map(Message::WindowClosed),
            iced::event::listen_with(|event, _status, id| match event {
                iced::Event::Window(window::Event::Focused) => Some(Message::WindowFocused(id)),
                _ => None,
            }),
            poll,
            retry,
        ])
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        match self.windows.get(&id) {
            Some(WindowKind::Layout) => layout_view::view(self),
            Some(WindowKind::Settings) => settings_view::view(self),
            None => iced::widget::text("").into(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Init => self.init(),
            Message::Device(DeviceMessage::Ready(handle)) => {
                self.device = Some(handle);
                Task::none()
            }
            Message::Device(DeviceMessage::Event(event)) => {
                if device::loggable(&event) {
                    tracing::debug!(?event, "device");
                }
                self.dispatch(Input::Device(event))
            }
            Message::Menu(id) => self.on_menu(&id),
            Message::SystemTheme(mode) => {
                // iced reports `None` before any window exists (Phase 0); keep the seeded mode then.
                // It also stops reporting changes once it pins a window's appearance, so the poll
                // re-seeds the mode too.
                if mode != Mode::None {
                    self.system_mode = mode;
                    // The Linux tray icon is drawn for the appearance.
                    self.refresh_tray();
                }
                Task::none()
            }
            Message::WindowOpened(id) => {
                platform::activate_app();
                window::gain_focus(id)
            }
            Message::WindowFocused(id) => {
                self.focused = Some(id);
                Task::none()
            }
            Message::WindowClosed(id) => {
                self.windows.remove(&id);
                if self.focused == Some(id) {
                    self.focused = None;
                }
                Task::none()
            }
            Message::PollSource => {
                self.sync_current_source();
                // iced misses system appearance changes once a window is open.
                self.seed_system_mode();
                Task::none()
            }
            Message::LayoutFetched {
                model,
                ids,
                from_cache,
                result,
            } => self.on_layout_fetched(model, ids, from_cache, result),
            Message::ChipPressed(layer) => self.dispatch(Input::PreviewLayer(Some(layer))),
            Message::FollowLive => self.dispatch(Input::PreviewLayer(None)),
            Message::SetLayerSource(layer, choice) => {
                let mut config = self.engine.config().clone();
                match choice.id() {
                    Some(id) => {
                        config.layers.insert(layer, id.to_owned());
                    }
                    None => {
                        config.layers.remove(&layer);
                    }
                }
                self.dispatch(Input::ConfigChanged(config))
            }
            Message::SetFallback(choice) => {
                let mut config = self.engine.config().clone();
                config.fallback = choice.id().map(str::to_owned);
                self.dispatch(Input::ConfigChanged(config))
            }
            Message::SetAppearance(appearance) => {
                let mut config = self.engine.config().clone();
                config.appearance = appearance;
                self.dispatch(Input::ConfigChanged(config))
            }
            Message::SetLaunchAtLogin(enabled) => match platform::set_launch_at_login(enabled) {
                Ok(()) => {
                    self.launch_error = None;
                    let mut config = self.engine.config().clone();
                    config.launch_at_login = enabled;
                    self.dispatch(Input::ConfigChanged(config))
                }
                Err(error) => {
                    tracing::warn!(%error, "could not change launch at login");
                    self.launch_error = Some(error);
                    Task::none()
                }
            },
            Message::RefreshLayout => self.dispatch(Input::RefreshLayout),
            Message::RetryLayout => self.dispatch(Input::RetryLayout),
        }
    }

    fn init(&mut self) -> Task<Message> {
        tray::install_event_handlers();
        platform::hide_dock_icon();
        self.seed_system_mode();
        match AppMenu::install() {
            Ok(menu) => self.app_menu = Some(menu),
            Err(error) => tracing::warn!(%error, "could not install the app menu"),
        }
        match Tray::new() {
            Ok(tray) => self.tray = Some(tray),
            Err(error) => tracing::error!(%error, "could not create the menu-bar icon"),
        }
        if self.engine.config().launch_at_login {
            match platform::refresh_launch_at_login() {
                Ok(true) => tracing::info!("updated the login item"),
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "could not refresh the login item"),
            }
        }
        self.refresh_sources();
        self.sync_current_source();
        self.refresh_tray();
        Task::none()
    }

    fn on_menu(&mut self, id: &str) -> Task<Message> {
        match tray::menu_action(id) {
            Some(MenuAction::ShowLayout) => self.open_window(WindowKind::Layout),
            Some(MenuAction::ShowSettings) => {
                self.refresh_sources();
                self.open_window(WindowKind::Settings)
            }
            Some(MenuAction::TogglePause) => {
                let paused = !self.engine.paused();
                self.dispatch(Input::SetPaused(paused))
            }
            Some(MenuAction::Quit) => self.dispatch(Input::Quit),
            Some(MenuAction::CloseWindow) => self.close_focused_window(),
            None => {
                tracing::debug!(id, "unknown menu item");
                Task::none()
            }
        }
    }

    /// ⌘W / ⌘Q in a window: close it and keep running in the menu bar.
    fn close_focused_window(&mut self) -> Task<Message> {
        let target = self
            .focused
            .filter(|id| self.windows.contains_key(id))
            .or_else(|| self.windows.keys().next_back().copied());
        match target {
            Some(id) => window::close(id),
            None => Task::none(),
        }
    }

    fn open_window(&mut self, kind: WindowKind) -> Task<Message> {
        if let Some((&id, _)) = self.windows.iter().find(|(_, k)| **k == kind) {
            platform::activate_app();
            // winit ignores a focus request while the window is minimized, so restore it first.
            return window::minimize(id, false).chain(window::gain_focus(id));
        }
        let settings = match kind {
            WindowKind::Layout => window::Settings {
                size: Size::new(980.0, 440.0),
                min_size: Some(Size::new(640.0, 300.0)),
                ..window::Settings::default()
            },
            WindowKind::Settings => window::Settings {
                size: Size::new(560.0, 720.0),
                min_size: Some(Size::new(480.0, 420.0)),
                ..window::Settings::default()
            },
        };
        // Read the appearance now, so the first frame already uses the right palette.
        self.seed_system_mode();
        let (id, open) = window::open(settings);
        self.windows.insert(id, kind);
        open.map(Message::WindowOpened)
    }

    fn seed_system_mode(&mut self) {
        let mode = platform::system_mode();
        if mode != Mode::None {
            self.system_mode = mode;
        }
    }

    fn dispatch(&mut self, input: Input) -> Task<Message> {
        let effects = self.engine.handle(input);
        let mut tasks = Vec::new();
        for effect in effects {
            match effect {
                Effect::SelectInputSource(id) => self.select_source(&id),
                Effect::FetchLayout { model, ids, force } => {
                    tasks.push(self.fetch_layout(model, ids, force))
                }
                Effect::PersistConfig => {
                    if let Err(error) = config::save(&self.paths.config_file, self.engine.config())
                    {
                        tracing::error!(%error, "could not save settings");
                    }
                }
                Effect::RefreshTray => self.refresh_tray(),
                Effect::Exit => {
                    if let Some(device) = &self.device {
                        device.shutdown();
                    }
                    QUIT_HANDLED.store(true, Ordering::SeqCst);
                    tasks.push(iced::exit());
                }
            }
        }
        Task::batch(tasks)
    }

    /// Spec §8: a source that can't be selected is skipped and logged once (until it works again).
    fn select_source(&mut self, id: &str) {
        match self.backend.select(id) {
            Ok(()) => {
                self.warned_missing.remove(id);
                tracing::info!(id, "selected input source");
            }
            Err(error) => {
                if self.warned_missing.insert(id.to_owned()) {
                    tracing::warn!(%error, id, "could not select input source");
                } else {
                    tracing::debug!(%error, id, "could not select input source (already reported)");
                }
                let _ = self.engine.handle(Input::SelectFailed(id.to_owned()));
            }
        }
        self.sync_current_source();
    }

    fn sync_current_source(&mut self) {
        let current = self.backend.current().ok();
        if current.as_deref() != self.engine.current_source() {
            let _ = self.engine.handle(Input::CurrentSourceChanged(current));
            self.refresh_tray();
        }
    }

    fn refresh_sources(&mut self) {
        match self.backend.list() {
            Ok(sources) => self.sources = sources,
            Err(error) => tracing::warn!(%error, "could not list input sources"),
        }
    }

    fn refresh_tray(&mut self) {
        let view = self.engine.tray_view(|id| self.source_name(id));
        let dark = self.is_dark();
        if let Some(tray) = self.tray.as_mut() {
            tray.apply(&view, dark);
        }
    }

    fn fetch_layout(&self, model: Model, ids: LayoutIds, force: bool) -> Task<Message> {
        if !force && let Some(json) = self.cache.load(&ids) {
            return Task::done(Message::LayoutFetched {
                model,
                ids,
                from_cache: true,
                result: Ok(json),
            });
        }
        let Some(http) = self.http.clone() else {
            return Task::done(Message::LayoutFetched {
                model,
                ids,
                from_cache: false,
                result: Err("HTTP client unavailable".into()),
            });
        };
        let request_ids = ids.clone();
        Task::perform(
            async move {
                client::fetch_layout(
                    &http,
                    client::ORYX_GRAPHQL_URL,
                    &request_ids,
                    model.oryx_geometry(),
                )
                .await
                .map_err(|e| e.to_string())
            },
            move |result| Message::LayoutFetched {
                model,
                ids,
                from_cache: false,
                result,
            },
        )
    }

    fn on_layout_fetched(
        &mut self,
        model: Model,
        ids: LayoutIds,
        from_cache: bool,
        result: Result<String, String>,
    ) -> Task<Message> {
        let parsed = match resolve_fetch(result, from_cache, || self.cache.load(&ids)) {
            FetchOutcome::Use {
                layout,
                store_json,
                fallback,
            } => {
                if let Some(json) = store_json
                    && let Err(error) = self.cache.store(&ids, &json)
                {
                    tracing::warn!(%error, "could not cache the layout");
                }
                if let Some(reason) = fallback {
                    tracing::info!(%reason, "using the cached layout");
                }
                Ok(layout)
            }
            FetchOutcome::Refetch => {
                tracing::warn!("the cached layout is unreadable; fetching it again");
                return self.fetch_layout(model, ids, true);
            }
            FetchOutcome::Fail(error) => {
                tracing::warn!(%error, "layout unavailable");
                Err(error)
            }
        };
        self.dispatch(Input::LayoutLoaded {
            ids,
            result: parsed,
        })
    }
}
