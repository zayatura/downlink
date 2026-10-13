//! Downlink: a menu-bar companion for the ZSA Voyager.

pub mod app;
pub mod icon;
pub mod logging;
pub mod paths;
pub mod platform;
pub mod single_instance;
pub mod source_choice;
pub mod theme;

mod device;
mod layout_view;
mod settings_view;
pub mod tray;

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

/// Safety net for exits that bypass Quit (spec §8), for example ⌘Q in a window.
fn select_fallback_on_exit(paths: &Paths) {
    let config = companion_core::config::load(&paths.config_file).config;
    if let Some(id) = config.fallback {
        match input_source::system_backend().select(&id) {
            Ok(()) => tracing::info!(id, "selected the fallback input source on exit"),
            Err(error) => {
                tracing::warn!(%error, "could not select the fallback input source on exit")
            }
        }
    }
}
