//! Built-in demo assets.

use anyhow::Context;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEXT: &str = include_str!("../examples/demo.txt");

const SLIDES_DEMO: &[u8] = include_bytes!("../examples/slides-demo.pptx");

pub fn write_slides_demo() -> anyhow::Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before UNIX_EPOCH")?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "termeprompter-slides-demo-{}-{now}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join("slides-demo.pptx");
    fs::write(&path, SLIDES_DEMO).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}
