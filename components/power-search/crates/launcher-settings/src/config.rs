use crate::color::Rgba;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse failed: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config serialization failed: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub general: General,
    pub appearance: Appearance,
    pub theme: ThemeSettings,
    pub animation: Animation,
    pub hotkey: Hotkey,
    pub prefixes: BTreeMap<String, String>,
    pub priorities: BTreeMap<String, i32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General::default(),
            appearance: Appearance::default(),
            theme: ThemeSettings::default(),
            animation: Animation::default(),
            hotkey: Hotkey::default(),
            prefixes: [
                (">", "system"),
                ("=", "calc"),
                ("f ", "files"),
                ("w ", "windows"),
                ("a ", "apps"),
            ]
            .into_iter()
            .map(|(prefix, module)| (prefix.to_owned(), module.to_owned()))
            .collect(),
            priorities: [
                ("apps", 40),
                ("calc", 30),
                ("windows", 20),
                ("files", 10),
                ("system", 5),
            ]
            .into_iter()
            .map(|(module, priority)| (module.to_owned(), priority))
            .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    pub max_visible_results: usize,
    pub candidate_limit: usize,
    pub auto_calculate: bool,
    pub close_on_launch: bool,
    pub close_on_blur: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            max_visible_results: 8,
            candidate_limit: 200,
            auto_calculate: true,
            close_on_launch: true,
            close_on_blur: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Blur {
    Off,
    #[default]
    #[serde(alias = "acrylic", alias = "mica")]
    Auto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub width: f32,
    pub opacity: f32,
    pub corner_radius: f32,
    pub blur: Blur,
    pub border: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            width: 640.0,
            opacity: 0.92,
            corner_radius: 16.0,
            blur: Blur::Auto,
            border: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CustomPalette {
    pub primary: Option<Rgba>,
    pub on_primary: Option<Rgba>,
    pub surface: Option<Rgba>,
    pub on_surface: Option<Rgba>,
    pub surface_container: Option<Rgba>,
    pub outline: Option<Rgba>,
    pub error: Option<Rgba>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeSettings {
    pub mode: ThemeMode,
    pub custom: CustomPalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    Disabled,
    Fast,
    #[default]
    Normal,
    Slow,
}

impl Speed {
    pub fn duration(self) -> Duration {
        match self {
            Speed::Disabled => Duration::ZERO,
            Speed::Fast => Duration::from_millis(120),
            Speed::Normal => Duration::from_millis(220),
            Speed::Slow => Duration::from_millis(420),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Animation {
    pub speed: Speed,
    pub list_expansion: bool,
    pub fade_in: bool,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            speed: Speed::Normal,
            list_expansion: true,
            fade_in: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Hotkey {
    pub summon: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            summon: String::from("Alt+Space"),
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("dev", "launcher", "launcher")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                return Ok(config);
            }
            Err(err) => return Err(err.into()),
        };
        let mut config: Self = toml::from_str(&text)?;
        config.validate();
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn validate(&mut self) {
        let general = &mut self.general;
        general.max_visible_results = general.max_visible_results.clamp(3, 20);
        general.candidate_limit = general.candidate_limit.clamp(20, 1_000);

        let appearance = &mut self.appearance;
        appearance.opacity = appearance.opacity.clamp(0.4, 1.0);
        appearance.corner_radius = appearance.corner_radius.clamp(0.0, 28.0);
        appearance.width = appearance.width.clamp(360.0, 1_200.0);

        self.prefixes
            .retain(|prefix, module| !prefix.is_empty() && !module.is_empty());
    }
}

impl Config {
    pub fn apply_overrides(&mut self, values: &toml::Table) {
        if let Some(n) = number(values, "max_visible_results") {
            self.general.max_visible_results = n as usize;
        }
        if let Some(b) = flag(values, "close_on_launch") {
            self.general.close_on_launch = b;
        }
        if let Some(b) = flag(values, "close_on_blur") {
            self.general.close_on_blur = b;
        }
        if let Some(n) = number(values, "opacity") {
            self.appearance.opacity = n as f32;
        }
        if let Some(text) = values.get("blur").and_then(|v| v.as_str()) {
            self.appearance.blur = match text {
                "off" => Blur::Off,
                _ => Blur::Auto,
            };
        }
        if let Some(text) = values.get("summon").and_then(|v| v.as_str()) {
            self.hotkey.summon = text.to_string();
        }
        self.validate();
    }
}

fn number(values: &toml::Table, key: &str) -> Option<f64> {
    let value = values.get(key)?;
    value.as_float().or_else(|| value.as_integer().map(|i| i as f64))
}

fn flag(values: &toml::Table, key: &str) -> Option<bool> {
    values.get(key)?.as_bool()
}

#[cfg(test)]
mod fidget_overrides {
    use super::{Blur, Config};

    #[test]
    fn applies_fidget_values() {
        let values: toml::Table = toml::from_str(
            "max_visible_results = 12.0\nopacity = 0.8\nblur = \"off\"\nsummon = \"Ctrl+Shift+Space\"\nclose_on_launch = false",
        )
        .unwrap();
        let mut config = Config::default();
        config.apply_overrides(&values);
        assert_eq!(config.general.max_visible_results, 12);
        assert!((config.appearance.opacity - 0.8).abs() < 1e-6);
        assert_eq!(config.appearance.blur, Blur::Off);
        assert_eq!(config.hotkey.summon, "Ctrl+Shift+Space");
        assert!(!config.general.close_on_launch);
    }
}
