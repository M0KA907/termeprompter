//! Local rich presentation rendering for Kitty graphics terminals.

use anyhow::Context;
use crossterm::{cursor::MoveTo, queue, style::Print};
use ratatui::layout::Rect;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Max seconds to wait on an external converter before giving up and falling
/// back to text. Prevents a hung soffice from freezing the whole UI.
const CONVERT_TIMEOUT_SECS: u64 = 25;

#[derive(Debug)]
pub struct Presentation {
    source_path: PathBuf,
    work_dir: PathBuf,
    slides: Vec<PathBuf>,
    current: usize,
    last_draw: Option<DrawState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DrawState {
    slide: usize,
    area: Rect,
}

impl Presentation {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if !is_presentation_path(path) {
            anyhow::bail!("unsupported presentation type");
        }
        if !kitty_graphics_available() {
            anyhow::bail!("Kitty graphics are required to display rich presentations locally");
        }

        let work_dir = unique_work_dir()?;
        fs::create_dir_all(&work_dir)
            .with_context(|| format!("failed to create {}", work_dir.display()))?;
        let pdf = render_pdf(path, &work_dir)?;
        let slides = render_png_slides(&pdf, &work_dir)?;
        if slides.is_empty() {
            anyhow::bail!("presentation rendered no slides");
        }

        Ok(Self {
            source_path: path.to_path_buf(),
            work_dir,
            slides,
            current: 0,
            last_draw: None,
        })
    }

    pub fn source_name(&self) -> String {
        self.source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("presentation")
            .to_string()
    }

    pub fn current_slide(&self) -> usize {
        self.current + 1
    }

    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    pub fn next(&mut self) {
        self.current = (self.current + 1).min(self.slides.len().saturating_sub(1));
    }

    pub fn previous(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    pub fn first(&mut self) {
        self.current = 0;
    }

    pub fn last(&mut self) {
        self.current = self.slides.len().saturating_sub(1);
    }

    pub fn clear<W: Write>(&mut self, out: &mut W) -> std::io::Result<()> {
        self.last_draw = None;
        clear_graphics(out)
    }

    pub fn draw<W: Write>(&mut self, out: &mut W, area: Rect) -> std::io::Result<()> {
        if area.width == 0 || area.height == 0 {
            return self.clear(out);
        }
        let state = DrawState {
            slide: self.current,
            area,
        };
        if self.last_draw == Some(state) {
            return Ok(());
        }

        self.clear(out)?;
        let Some(path) = self.slides.get(self.current) else {
            return Ok(());
        };
        let (cols, rows) = image_cell_size(path, area).unwrap_or((area.width, area.height));
        let x = area.x + area.width.saturating_sub(cols) / 2;
        let y = area.y + area.height.saturating_sub(rows) / 2;
        let payload = base64(path.to_string_lossy().as_bytes());
        // q=2 suppresses Kitty's APC response. Without it Kitty echoes
        // `\x1b_Gi=7;OK\x1b\\`, which crossterm reads off stdin as stray key
        // events — the `i` in that reply fires OpenImport and pops the file
        // browser over the slide the instant it renders.
        let command = format!("\x1b_Ga=T,t=f,f=100,i=7,q=2,c={cols},r={rows};{payload}\x1b\\");
        queue!(out, MoveTo(x, y), Print(command))?;
        out.flush()?;
        self.last_draw = Some(state);
        Ok(())
    }
}

impl Drop for Presentation {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.work_dir);
    }
}

/// Delete every Kitty graphics placement on screen. Safe to call even when no
/// image was drawn (and harmless on terminals that ignore the protocol). Used
/// to wipe stale slide images so they never bleed under the import browser.
pub fn clear_graphics<W: Write>(out: &mut W) -> std::io::Result<()> {
    // q=2: suppress Kitty's response so it never leaks into stdin as keys.
    queue!(out, Print("\x1b_Ga=d,d=A,q=2\x1b\\"))?;
    out.flush()
}

pub fn is_presentation_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ppt" | "pptx" | "pps" | "ppsx"
            )
        })
        .unwrap_or(false)
}

pub fn kitty_graphics_available() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM")
            .map(|term| term.contains("kitty"))
            .unwrap_or(false)
}

fn unique_work_dir() -> anyhow::Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before UNIX_EPOCH")?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("termeprompter-slides-{}-{now}", std::process::id())))
}

fn render_pdf(path: &Path, out_dir: &Path) -> anyhow::Result<PathBuf> {
    // Isolated profile so we never block on a lock held by the user's own
    // running LibreOffice instance (the usual cause of a hung headless convert).
    let profile = std::env::temp_dir().join("termeprompter-loprofile");
    let mut cmd = Command::new("soffice");
    cmd.arg(format!(
        "-env:UserInstallation=file://{}",
        profile.display()
    ))
    .arg("--headless")
    .arg("--norestore")
    .arg("--convert-to")
    .arg("pdf")
    .arg("--outdir")
    .arg(out_dir)
    .arg(path);
    run_with_timeout(
        cmd,
        CONVERT_TIMEOUT_SECS,
        "soffice; install LibreOffice to render presentations",
    )?;

    let expected = out_dir
        .join(path.file_stem().unwrap_or_default())
        .with_extension("pdf");
    if expected.exists() {
        return Ok(expected);
    }
    find_with_extension(out_dir, "pdf").context("LibreOffice did not produce a PDF")
}

fn render_png_slides(pdf: &Path, out_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let prefix = out_dir.join("slide");
    let mut cmd = Command::new("pdftoppm");
    cmd.arg("-png").arg("-r").arg("144").arg(pdf).arg(&prefix);
    run_with_timeout(
        cmd,
        CONVERT_TIMEOUT_SECS,
        "pdftoppm; install poppler to render slide images",
    )?;

    let mut slides = fs::read_dir(out_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    slides.sort_by_key(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.rsplit('-').next())
            .and_then(|num| num.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    });
    Ok(slides)
}

/// Run `cmd` to completion but kill it (and bail) if it exceeds `secs`.
/// Output is discarded; callers verify success by inspecting produced files.
fn run_with_timeout(mut cmd: Command, secs: u64, what: &str) -> anyhow::Result<()> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to run {what}"))?;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait()? {
            Some(status) if status.success() => return Ok(()),
            Some(_) => anyhow::bail!("{what} failed"),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("{what} timed out after {secs}s");
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn find_with_extension(dir: &Path, extension: &str) -> anyhow::Result<PathBuf> {
    fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case(extension))
                .unwrap_or(false)
        })
        .with_context(|| format!("no .{extension} file in {}", dir.display()))
}

fn image_cell_size(path: &Path, area: Rect) -> Option<(u16, u16)> {
    let (width, height) = png_size(path)?;
    let target_ratio = (width as f64 / height as f64) * 2.0;
    let area_ratio = area.width as f64 / area.height.max(1) as f64;
    if area_ratio > target_ratio {
        let rows = area.height;
        let cols = ((rows as f64 * target_ratio).round() as u16).clamp(1, area.width);
        Some((cols, rows))
    } else {
        let cols = area.width;
        let rows = ((cols as f64 / target_ratio).round() as u16).clamp(1, area.height);
        Some((cols, rows))
    }
}

fn png_size(path: &Path) -> Option<(u32, u32)> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_graphics_suppresses_kitty_response() {
        // q=2 must be present or Kitty echoes an OK reply that crossterm
        // mis-reads as keystrokes (the slides-mode phantom file browser).
        let mut out = Vec::new();
        clear_graphics(&mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("q=2"), "clear_graphics missing q=2: {s:?}");
        assert!(s.contains("a=d"));
    }
}

fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
