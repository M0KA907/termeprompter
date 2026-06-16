//! Import browser and local text loading.

use crate::document::Document;
use crate::parser;
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind {
    Folder,
    Text,
    PowerPoint,
    Slideshow,
}

#[derive(Clone, Debug)]
pub struct ImportEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Clone, Debug)]
pub struct ImportMenu {
    pub cwd: PathBuf,
    pub entries: Vec<ImportEntry>,
    pub selected: usize,
    pub message: Option<String>,
}

impl ImportMenu {
    pub fn open() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut menu = Self {
            cwd,
            entries: Vec::new(),
            selected: 0,
            message: None,
        };
        menu.refresh();
        menu
    }

    pub fn refresh(&mut self) {
        match list_entries(&self.cwd) {
            Ok(entries) => {
                self.entries = entries;
                self.selected = self.selected.min(self.entries.len());
                self.message = None;
            }
            Err(err) => {
                self.entries.clear();
                self.selected = 0;
                self.message = Some(format!("Cannot read folder: {err}"));
            }
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let max = self.entries.len();
        self.selected = self.selected.saturating_add_signed(delta).min(max);
    }

    pub fn parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = 0;
            self.refresh();
        }
    }
}

pub fn load_import(path: &Path) -> anyhow::Result<Document> {
    if path.is_dir() {
        return load_folder(path);
    }

    match kind_for_path(path) {
        Some(ImportKind::Text) => load_text_file(path),
        Some(ImportKind::PowerPoint) | Some(ImportKind::Slideshow) => load_ooxml_slides(path),
        _ => anyhow::bail!("unsupported import type"),
    }
}

fn load_text_file(path: &Path) -> anyhow::Result<Document> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut doc = parser::parse(&input);
    doc.source_path = Some(path.to_path_buf());
    Ok(doc)
}

fn load_folder(path: &Path) -> anyhow::Result<Document> {
    let mut files = Vec::new();
    collect_text_files(path, &mut files)?;
    if files.is_empty() {
        anyhow::bail!("folder has no supported text files");
    }

    let mut input = String::new();
    for file in files {
        let body = fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let label = file.strip_prefix(path).unwrap_or(&file).display();
        input.push_str(&format!("\n# {label}\n\n{body}\n"));
    }

    let mut doc = parser::parse(input.trim_start());
    doc.source_path = Some(path.to_path_buf());
    Ok(doc)
}

fn load_ooxml_slides(path: &Path) -> anyhow::Result<Document> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(ext.as_str(), "ppt" | "pps") {
        anyhow::bail!(
            "legacy binary .{ext} files are not supported; save as .pptx or .ppsx and try again"
        );
    }

    let entries = zip_entries(path)?;
    let mut slides = entries
        .iter()
        .filter_map(|entry| numbered_ooxml_entry(entry, "ppt/slides/slide", ".xml"))
        .collect::<Vec<_>>();
    slides.sort_by_key(|(number, _)| *number);

    if slides.is_empty() {
        anyhow::bail!("PowerPoint file has no readable slides");
    }

    let mut notes = entries
        .iter()
        .filter_map(|entry| numbered_ooxml_entry(entry, "ppt/notesSlides/notesSlide", ".xml"))
        .collect::<Vec<_>>();
    notes.sort_by_key(|(number, _)| *number);

    let mut input = String::new();
    for (slide_number, slide_entry) in slides {
        let slide_xml = unzip_entry(path, slide_entry)?;
        let slide_lines = extract_ooxml_text(&slide_xml);
        input.push_str(&format!("# Slide {slide_number}\n\n"));
        append_lines(&mut input, &slide_lines);

        if let Some((_, notes_entry)) = notes.iter().find(|(number, _)| *number == slide_number) {
            let notes_xml = unzip_entry(path, notes_entry)?;
            let notes_lines = extract_ooxml_text(&notes_xml);
            if !notes_lines.is_empty() {
                input.push_str("\n## Speaker notes\n\n");
                append_lines(&mut input, &notes_lines);
            }
        }

        input.push_str("\n---\n\n");
    }

    let mut doc = parser::parse(input.trim_end());
    doc.source_path = Some(path.to_path_buf());
    Ok(doc)
}

fn collect_text_files(path: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_text_files(&path, files)?;
        } else if matches!(kind_for_path(&path), Some(ImportKind::Text)) {
            files.push(path);
        }
    }
    Ok(())
}

fn zip_entries(path: &Path) -> anyhow::Result<Vec<String>> {
    let output = Command::new("unzip")
        .arg("-Z1")
        .arg(path)
        .output()
        .with_context(|| "failed to run unzip; install unzip or export the deck as text")?;
    if !output.status.success() {
        anyhow::bail!("failed to inspect PowerPoint archive with unzip");
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn unzip_entry(path: &Path, entry: &str) -> anyhow::Result<String> {
    let output = Command::new("unzip")
        .arg("-p")
        .arg(path)
        .arg(entry)
        .output()
        .with_context(|| format!("failed to extract {entry} with unzip"))?;
    if !output.status.success() {
        anyhow::bail!("failed to extract {entry} from PowerPoint archive");
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn numbered_ooxml_entry<'a>(
    entry: &'a str,
    prefix: &str,
    suffix: &str,
) -> Option<(usize, &'a str)> {
    entry
        .strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse::<usize>()
        .ok()
        .map(|number| (number, entry))
}

fn extract_ooxml_text(xml: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<a:p") {
        rest = &rest[start..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        rest = &rest[open_end + 1..];
        let Some(close) = rest.find("</a:p>") else {
            break;
        };
        let paragraph = &rest[..close];
        let text = extract_text_runs(paragraph);
        if !text.trim().is_empty() {
            lines.push(text);
        }
        rest = &rest[close + "</a:p>".len()..];
    }
    lines
}

fn extract_text_runs(xml: &str) -> String {
    let mut text = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<a:t") {
        rest = &rest[start..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        rest = &rest[open_end + 1..];
        let Some(close) = rest.find("</a:t>") else {
            break;
        };
        text.push_str(&decode_xml_entities(&rest[..close]));
        rest = &rest[close + "</a:t>".len()..];
    }
    text
}

fn append_lines(input: &mut String, lines: &[String]) {
    for line in lines {
        input.push_str(line);
        input.push('\n');
    }
}

fn decode_xml_entities(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find('&') {
        output.push_str(&rest[..idx]);
        rest = &rest[idx + 1..];
        let Some(end) = rest.find(';') else {
            output.push('&');
            output.push_str(rest);
            return output;
        };
        let entity = &rest[..end];
        match entity {
            "amp" => output.push('&'),
            "lt" => output.push('<'),
            "gt" => output.push('>'),
            "apos" => output.push('\''),
            "quot" => output.push('"'),
            _ if entity.starts_with("#x") => {
                if let Ok(value) = u32::from_str_radix(&entity[2..], 16) {
                    if let Some(ch) = char::from_u32(value) {
                        output.push(ch);
                    }
                }
            }
            _ if entity.starts_with('#') => {
                if let Ok(value) = entity[1..].parse::<u32>() {
                    if let Some(ch) = char::from_u32(value) {
                        output.push(ch);
                    }
                }
            }
            _ => {
                output.push('&');
                output.push_str(entity);
                output.push(';');
            }
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}

fn list_entries(path: &Path) -> anyhow::Result<Vec<ImportEntry>> {
    let mut entries = fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = entry.file_type().ok()?;
            let kind = if file_type.is_dir() {
                ImportKind::Folder
            } else {
                kind_for_path(&path)?
            };
            let name = entry.file_name().to_string_lossy().to_string();
            Some(ImportEntry { path, name, kind })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_dir = matches!(a.kind, ImportKind::Folder);
        let b_dir = matches!(b.kind, ImportKind::Folder);
        b_dir.cmp(&a_dir).then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

fn kind_for_path(path: &Path) -> Option<ImportKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "text" | "md" | "markdown" | "rst" | "adoc" | "asc" => Some(ImportKind::Text),
        "ppt" | "pptx" => Some(ImportKind::PowerPoint),
        "pps" | "ppsx" => Some(ImportKind::Slideshow),
        _ => None,
    }
}
