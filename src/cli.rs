//! CLI parsing.

use crate::render::LayoutKind;
use crate::scroll::{WPM_MAX, WPM_MIN};
use crate::theme::ThemeKind;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Cli {
    pub path: Option<PathBuf>,
    pub demo: bool,
    pub wpm: Option<u16>,
    pub theme: Option<ThemeKind>,
    pub layout: Option<LayoutKind>,
    pub mirror: bool,
    pub ascii: bool,
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct RawCli {
    /// Script file to read.
    path: Option<PathBuf>,
    /// Use built-in demo text.
    #[arg(long)]
    demo: bool,
    /// Words per minute. Must be 40..=1000.
    #[arg(long)]
    wpm: Option<u16>,
    /// Theme: rose-plum, plain, mono, high-contrast.
    #[arg(long)]
    theme: Option<String>,
    /// Layout: horizontal, trainer, prompt, rehearsal, minimal.
    #[arg(long)]
    layout: Option<String>,
    /// Start mirrored.
    #[arg(long)]
    mirror: bool,
    /// Use ASCII-only chrome.
    #[arg(long)]
    ascii: bool,
}

impl Cli {
    pub fn parse_args() -> anyhow::Result<Self> {
        let raw = RawCli::parse();
        if let Some(wpm) = raw.wpm {
            if !(WPM_MIN..=WPM_MAX).contains(&wpm) {
                anyhow::bail!("--wpm must be in {WPM_MIN}..={WPM_MAX}, got {wpm}");
            }
        }
        let theme = raw
            .theme
            .as_deref()
            .map(ThemeKind::from_str)
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let layout = raw
            .layout
            .as_deref()
            .map(LayoutKind::from_str)
            .transpose()
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            path: raw.path,
            demo: raw.demo,
            wpm: raw.wpm,
            theme,
            layout,
            mirror: raw.mirror,
            ascii: raw.ascii,
        })
    }
}
