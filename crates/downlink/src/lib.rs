//! Downlink: a menu-bar companion for the ZSA Voyager.

pub mod app;
pub mod icon;
pub mod layout_fetch;
pub mod logging;
pub mod paths;
pub mod platform;
pub mod single_instance;
pub mod source_choice;
pub mod theme;

mod device;
pub mod layout_view;
pub mod settings_view;
pub mod tray;

use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::Ordering;

use app::App;
use paths::Paths;

/// Runs Downlink until the user quits.
pub fn run() -> ExitCode {
    let Some(paths) = Paths::detect() else {
        eprintln!("Downlink: could not determine the home and config directories");
        return ExitCode::FAILURE;
    };
    let _log_guard = logging::init(&paths.log_dir);
    let _lock = match single_instance::acquire(&paths.lock_file) {
        Ok(Some(lock)) => lock,
        Ok(None) => {
            tracing::info!("Downlink is already running; exiting");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            tracing::error!(%error, "could not create the single-instance lock");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Downlink starting");
    let boot_paths = paths.clone();
    let result = iced::daemon(
        move || App::boot(boot_paths.clone()),
        App::update,
        App::view,
    )
    .title(App::title)
    .theme(App::theme)
    .subscription(App::subscription)
    .run();
    if !app::QUIT_HANDLED.load(Ordering::SeqCst) {
        select_fallback_on_exit(&paths);
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "the event loop failed");
            ExitCode::FAILURE
        }
    }
}

/// Safety net for event-loop exits that bypass Quit (spec §8), e.g. an event-loop error.
fn select_fallback_on_exit(paths: &Paths) {
    let config = load_config_logged(&paths.config_file);
    if let Some(id) = config.fallback {
        match input_source::system_backend().select(&id) {
            Ok(()) => tracing::info!(id, "selected the fallback input source on exit"),
            Err(error) => {
                tracing::warn!(%error, "could not select the fallback input source on exit")
            }
        }
    }
}

/// Loads the settings and logs each warning. A hand-edited file may be moved aside as
/// `config.toml.bad-<secs>` by this load, and the warning is the only record of it.
pub fn load_config_logged(path: &Path) -> companion_core::Config {
    let loaded = companion_core::config::load(path);
    for warning in &loaded.warnings {
        tracing::warn!("{warning}");
    }
    loaded.config
}

/// Like `load_config_logged`, but on the first run (the file doesn't exist) it also writes the
/// defaults, so there is a file to edit (spec §7). It never writes after a file that exists but
/// couldn't be read or parsed: that file belongs to the user, and the load moved it aside.
pub fn load_or_create_config(path: &Path) -> companion_core::Config {
    let first_run = matches!(path.try_exists(), Ok(false));
    let config = load_config_logged(path);
    if first_run {
        match companion_core::config::save(path, &config) {
            Ok(()) => tracing::info!("created the settings file with defaults"),
            Err(error) => tracing::warn!(%error, "could not create the settings file"),
        }
    }
    config
}
