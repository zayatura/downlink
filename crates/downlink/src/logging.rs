use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Overridable with the `DOWNLINK_LOG` environment variable (EnvFilter syntax).
pub const DEFAULT_FILTER: &str =
    "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,iced_wgpu=warn,cosmic_text=warn";

/// Logs to daily-rotated files in `log_dir` (7 kept) and to stderr. Keep the guard alive.
pub fn init(log_dir: &Path) -> Option<WorkerGuard> {
    let filter =
        EnvFilter::try_from_env("DOWNLINK_LOG").unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let appender = std::fs::create_dir_all(log_dir)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            Builder::new()
                .rotation(Rotation::DAILY)
                .filename_prefix("downlink")
                .filename_suffix("log")
                .max_log_files(7)
                .build(log_dir)
                .map_err(|e| e.to_string())
        });
    let (file_layer, guard) = match appender {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                Some(fmt::layer().with_ansi(false).with_writer(writer)),
                Some(guard),
            )
        }
        Err(error) => {
            eprintln!("Downlink: file logging disabled ({error})");
            (None, None)
        }
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
    guard
}
