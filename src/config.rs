//! Optional user config.

use crate::render::LayoutKind;
use crate::scroll::WPM_DEFAULT;
use crate::theme::ThemeKind;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Config {
    pub wpm: u16,
    pub theme: ThemeKind,
    pub layout: LayoutKind,
    pub mirror: bool,
    pub ascii: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wpm: WPM_DEFAULT,
            theme: ThemeKind::default(),
            layout: LayoutKind::default(),
            mirror: false,
            ascii: false,
        }
    }
}

#[derive(Deserialize, Default)]
struct RawConfig {
    wpm: Option<u16>,
    theme: Option<String>,
    layout: Option<String>,
    mirror: Option<bool>,
    ascii: Option<bool>,
}

impl Config {
    pub fn load_default() -> anyhow::Result<Self> {
        let Some(path) = default_path() else {
            return Ok(Self::default());
        };
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw: RawConfig = toml::from_str(&fs::read_to_string(path)?)?;
        let mut cfg = Self::default();
        if let Some(wpm) = raw.wpm {
            cfg.wpm = wpm;
        }
        if let Some(theme) = raw.theme {
            cfg.theme = ThemeKind::from_str(&theme).map_err(anyhow::Error::msg)?;
        }
        if let Some(layout) = raw.layout {
            cfg.layout = LayoutKind::from_str(&layout).map_err(anyhow::Error::msg)?;
        }
        if let Some(mirror) = raw.mirror {
            cfg.mirror = mirror;
        }
        if let Some(ascii) = raw.ascii {
            cfg.ascii = ascii;
        }
        Ok(cfg)
    }
}

fn default_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("termeprompter").join("config.toml"))
}
