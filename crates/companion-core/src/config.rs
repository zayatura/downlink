//! The settings file, `config.toml` (spec §7). Parsing is lenient: every
//! invalid part is skipped with a warning and the valid parts are kept.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

use toml::{Table, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub fn as_str(self) -> &'static str {
        match self {
            Appearance::System => "system",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        }
    }

    fn from_name(name: &str) -> Option<Appearance> {
        match name {
            "system" => Some(Appearance::System),
            "light" => Some(Appearance::Light),
            "dark" => Some(Appearance::Dark),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub appearance: Appearance,
    pub launch_at_login: bool,
    /// Input source to select on disconnect and quit; `None` = no change.
    pub fallback: Option<String>,
    /// Layer index → input-source ID. Layers not listed are transient.
    pub layers: BTreeMap<u8, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub config: Config,
    pub warnings: Vec<String>,
}

pub fn load(path: &Path) -> Loaded {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Loaded {
                config: Config::default(),
                warnings: Vec::new(),
            };
        }
        Err(error) => {
            return Loaded {
                config: Config::default(),
                warnings: vec![format!(
                    "could not read {}: {error}; using defaults",
                    path.display()
                )],
            };
        }
    };
    match parse(&text) {
        Ok((config, warnings)) => Loaded { config, warnings },
        Err(error) => {
            let backup = backup_path(path, SystemTime::now());
            let note = match fs::rename(path, &backup) {
                Ok(()) => format!("kept the unreadable file as {}", backup.display()),
                Err(rename_error) => format!("could not move it aside: {rename_error}"),
            };
            Loaded {
                config: Config::default(),
                warnings: vec![format!(
                    "{} is not valid TOML ({error}); using defaults and {note}",
                    path.display()
                )],
            }
        }
    }
}

pub fn save(path: &Path, config: &Config) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, to_toml(config))?;
    fs::rename(&tmp, path)
}

pub fn backup_path(path: &Path, now: SystemTime) -> PathBuf {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".into());
    path.with_file_name(format!("{name}.bad-{secs}"))
}

/// Returns the config plus one warning per ignored part. Errs only on invalid TOML.
pub fn parse(text: &str) -> Result<(Config, Vec<String>), String> {
    let table: Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let mut config = Config::default();
    let mut warnings = Vec::new();
    match table.get("appearance") {
        None => {}
        Some(Value::String(name)) => match Appearance::from_name(name) {
            Some(appearance) => config.appearance = appearance,
            None => warnings.push(format!("unknown appearance {name:?}; using \"system\"")),
        },
        Some(other) => warnings.push(format!(
            "appearance must be a string, found {}",
            other.type_str()
        )),
    }
    match table.get("launch_at_login") {
        None => {}
        Some(Value::Boolean(enabled)) => config.launch_at_login = *enabled,
        Some(other) => warnings.push(format!(
            "launch_at_login must be true or false, found {}",
            other.type_str()
        )),
    }
    match table.get("switching") {
        None => {}
        Some(Value::Table(switching)) => read_switching(switching, &mut config, &mut warnings),
        Some(other) => warnings.push(format!(
            "[switching] must be a table, found {}",
            other.type_str()
        )),
    }
    Ok((config, warnings))
}

fn read_switching(switching: &Table, config: &mut Config, warnings: &mut Vec<String>) {
    match switching.get("fallback") {
        None => {}
        Some(Value::String(id)) => {
            config.fallback = Some(id.trim().to_owned()).filter(|id| !id.is_empty())
        }
        Some(other) => warnings.push(format!(
            "switching.fallback must be a string, found {}",
            other.type_str()
        )),
    }
    match switching.get("layers") {
        None => {}
        Some(Value::Table(layers)) => {
            for (key, value) in layers {
                let Ok(index) = key.trim().parse::<u8>() else {
                    warnings.push(format!(
                        "ignoring [switching.layers] entry {key:?}: not a layer number 0-255"
                    ));
                    continue;
                };
                match value.as_str().map(str::trim) {
                    Some(id) if !id.is_empty() => {
                        config.layers.insert(index, id.to_owned());
                    }
                    _ => warnings.push(format!(
                        "ignoring [switching.layers] entry {key:?}: the value must be a non-empty input-source ID"
                    )),
                }
            }
        }
        Some(other) => warnings.push(format!(
            "[switching.layers] must be a table, found {}",
            other.type_str()
        )),
    }
}

pub fn to_toml(config: &Config) -> String {
    let mut switching = Table::new();
    if let Some(id) = &config.fallback {
        switching.insert("fallback".into(), Value::String(id.clone()));
    }
    let layers: Table = config
        .layers
        .iter()
        .map(|(index, id)| (index.to_string(), Value::String(id.clone())))
        .collect();
    switching.insert("layers".into(), Value::Table(layers));
    let mut root = Table::new();
    root.insert(
        "appearance".into(),
        Value::String(config.appearance.as_str().into()),
    );
    root.insert(
        "launch_at_login".into(),
        Value::Boolean(config.launch_at_login),
    );
    root.insert("switching".into(), Value::Table(switching));
    root.to_string()
}
