//! Plain text plus small markdown parsing.

use crate::document::{CuePoint, Document, Line, LineKind};
use crate::scroll::MIN_LINE_WEIGHT;

pub fn parse(input: &str) -> Document {
    let mut lines = Vec::new();
    let mut cues = Vec::new();

    for raw in input.lines() {
        let trimmed = raw.trim();
        let kind = classify_line(trimmed);
        let line_idx = lines.len();
        if let Some(label) = cue_label(trimmed) {
            cues.push(CuePoint {
                line_idx,
                label: label.to_string(),
            });
        }
        let words = count_words(raw).max(MIN_LINE_WEIGHT);
        lines.push(Line {
            kind,
            text: raw.to_string(),
            words,
        });
    }

    if input.is_empty() {
        lines.push(Line {
            kind: LineKind::Blank,
            text: String::new(),
            words: MIN_LINE_WEIGHT,
        });
    }

    let total_words = lines.iter().map(|line| line.words).sum();
    Document {
        source_path: None,
        lines,
        cues,
        total_words,
        version: 1,
    }
}

pub fn count_words(text: &str) -> f64 {
    text.split_whitespace()
        .filter(|word| !word.is_empty())
        .count() as f64
}

fn classify_line(trimmed: &str) -> LineKind {
    if trimmed.is_empty() {
        return LineKind::Blank;
    }
    if matches!(trimmed, "---" | "***" | "___") {
        return LineKind::Rule;
    }
    if cue_label(trimmed).is_some() {
        return LineKind::Cue;
    }
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ') {
        return LineKind::Heading(hashes as u8);
    }
    LineKind::Body
}

fn cue_label(trimmed: &str) -> Option<&str> {
    trimmed
        .strip_prefix("[cue:")
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim)
        .filter(|label| !label.is_empty())
}
