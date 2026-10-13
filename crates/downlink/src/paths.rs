use std::path::{Path, PathBuf};

/// Where Downlink keeps its files (spec §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_file: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub lock_file: PathBuf,
}

impl Paths {
    pub fn macos(home: &Path) -> Paths {
        let library = home.join("Library");
        let cache_dir = library.join("Caches/Downlink");
        Paths {
            config_file: library.join("Application Support/Downlink/config.toml"),
            lock_file: cache_dir.join("downlink.lock"),
            cache_dir,
            log_dir: library.join("Logs/Downlink"),
        }
    }

    pub fn xdg(config: &Path, cache: &Path, state: &Path, runtime: Option<&Path>) -> Paths {
        let cache_dir = cache.join("downlink");
        Paths {
            config_file: config.join("downlink/config.toml"),
            lock_file: runtime
                .map(|dir| dir.join("downlink.lock"))
                .unwrap_or_else(|| cache_dir.join("downlink.lock")),
            cache_dir,
            log_dir: state.join("downlink"),
        }
    }

    pub fn detect() -> Option<Paths> {
        if cfg!(target_os = "macos") {
            Some(Paths::macos(&dirs::home_dir()?))
        } else {
            let state = dirs::state_dir().or_else(dirs::data_local_dir)?;
            Some(Paths::xdg(
                &dirs::config_dir()?,
                &dirs::cache_dir()?,
                &state,
                dirs::runtime_dir().as_deref(),
            ))
        }
    }

    pub fn layout_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("layouts")
    }
}
