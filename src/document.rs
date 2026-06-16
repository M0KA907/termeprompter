//! Loaded script: immutable source of truth. Logical lines, headings, cue points.
//! `text` is raw source and is NEVER mutated or mirrored.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Document {
    pub source_path: Option<PathBuf>,
    pub lines: Vec<Line>,
    pub cues: Vec<CuePoint>,
    pub total_words: f64,
    pub version: u64,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    /// Word-equivalent weight, always >= MIN_LINE_WEIGHT.
    pub words: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineKind {
    Body,
    Heading(u8),
    Cue,
    Blank,
    Rule,
}

#[derive(Clone, Debug)]
pub struct CuePoint {
    pub line_idx: usize,
    pub label: String,
}

impl Document {
    /// First logical-line index for the heading currently in effect at `line_idx`.
    pub fn current_heading(&self, line_idx: usize) -> Option<&Line> {
        let capped = line_idx.min(self.lines.len().saturating_sub(1));
        self.lines[..=capped]
            .iter()
            .rev()
            .find(|line| matches!(line.kind, LineKind::Heading(_)))
    }
}
