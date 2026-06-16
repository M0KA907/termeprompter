//! Canonical word-axis scroll state and wrapped layout.

use crate::document::{Document, LineKind};
use crate::parser::count_words;
use crate::timing::Clock;
use std::time::Instant;
use unicode_width::UnicodeWidthChar;

pub const WPM_DEFAULT: u16 = 135;
pub const WPM_MIN: u16 = 40;
pub const WPM_MAX: u16 = 1000;
pub const MIN_ROW_WEIGHT: f64 = 0.35;
pub const MIN_LINE_WEIGHT: f64 = 0.35;
const EPS: f64 = 1e-6;

#[derive(Clone, Debug)]
pub struct WrapLayout {
    pub width: u16,
    pub doc_version: u64,
    pub rows: Vec<VisualRow>,
    pub cum_words: Vec<f64>,
    pub total_words: f64,
    pub total_rows: usize,
}

#[derive(Clone, Debug)]
pub struct VisualRow {
    pub line_idx: usize,
    pub byte_range: (usize, usize),
    pub kind: LineKind,
    pub words: f64,
}

impl WrapLayout {
    pub fn build(doc: &Document, content_width: u16) -> Self {
        let width = content_width.max(1);
        let mut rows = Vec::new();
        for (line_idx, line) in doc.lines.iter().enumerate() {
            for (start, end) in wrap_ranges(&line.text, width) {
                let segment = &line.text[start..end];
                rows.push(VisualRow {
                    line_idx,
                    byte_range: (start, end),
                    kind: line.kind,
                    words: count_words(segment).max(MIN_ROW_WEIGHT),
                });
            }
        }

        let mut cum_words = Vec::with_capacity(rows.len() + 1);
        cum_words.push(0.0);
        let mut total_words = 0.0;
        for row in &rows {
            total_words += row.words.max(MIN_ROW_WEIGHT);
            cum_words.push(total_words);
        }

        Self {
            width,
            doc_version: doc.version,
            total_rows: rows.len(),
            rows,
            cum_words,
            total_words,
        }
    }

    pub fn rows_to_words(&self, scroll_rows: f64) -> f64 {
        if self.total_rows == 0 {
            return 0.0;
        }
        let row_pos = finite_or(scroll_rows, 0.0).clamp(0.0, self.total_rows as f64);
        if row_pos >= self.total_rows as f64 {
            return self.total_words;
        }
        let row = row_pos.floor() as usize;
        let frac = row_pos - row as f64;
        let start = self.cum_words[row];
        let span = self.cum_words[row + 1] - start;
        start + span * frac
    }

    pub fn words_to_rows(&self, word_pos: f64) -> f64 {
        if self.total_rows == 0 || self.total_words <= 0.0 {
            return 0.0;
        }
        let word = finite_or(word_pos, 0.0).clamp(0.0, self.total_words);
        if word >= self.total_words {
            return self.total_rows as f64;
        }
        let idx = self.cum_words.partition_point(|&v| v <= word);
        let row = idx.saturating_sub(1).min(self.total_rows.saturating_sub(1));
        let start = self.cum_words[row];
        let end = self.cum_words[row + 1];
        let span = (end - start).max(MIN_ROW_WEIGHT);
        row as f64 + ((word - start) / span).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wpm(u16);

impl Wpm {
    pub fn new(v: u16) -> Self {
        Self(((v as i32).clamp(WPM_MIN as i32, WPM_MAX as i32)) as u16)
    }

    pub fn nudge(self, delta: i32) -> Self {
        Self(((self.0 as i32 + delta).clamp(WPM_MIN as i32, WPM_MAX as i32)) as u16)
    }

    pub fn get(self) -> u16 {
        self.0
    }

    pub fn words_per_sec(self) -> f64 {
        self.0 as f64 / 60.0
    }
}

impl Default for Wpm {
    fn default() -> Self {
        Self(WPM_DEFAULT)
    }
}

#[derive(Clone, Debug)]
pub struct ScrollState {
    word_pos: f64,
    anchor_word_pos: f64,
    play_anchor: Option<Instant>,
    wpm: Wpm,
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            word_pos: 0.0,
            anchor_word_pos: 0.0,
            play_anchor: None,
            wpm: Wpm::default(),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.play_anchor.is_some()
    }

    pub fn wpm(&self) -> Wpm {
        self.wpm
    }

    pub fn word_pos(&self) -> f64 {
        self.word_pos
    }

    pub fn at_end(&self, layout: &WrapLayout, viewport_rows: u16) -> bool {
        let _ = viewport_rows;
        self.word_pos >= max_word_pos(layout) - EPS
    }

    pub fn tick<C: Clock>(&mut self, clock: &C, layout: &WrapLayout, viewport_rows: u16) {
        if let Some(anchor) = self.play_anchor {
            let dt = clock.now().saturating_duration_since(anchor).as_secs_f64();
            let raw = self.anchor_word_pos + self.wpm.words_per_sec() * dt;
            self.word_pos = finite_or(raw, self.anchor_word_pos);
        }
        self.clamp_to(layout, viewport_rows);
        if self.play_anchor.is_some() && self.at_end(layout, viewport_rows) {
            self.reseat_paused();
        }
    }

    pub fn scroll_rows(&self, layout: &WrapLayout, viewport_rows: u16) -> f64 {
        let _ = viewport_rows;
        layout
            .words_to_rows(self.word_pos)
            .clamp(0.0, max_active_rows(layout))
    }

    pub fn pause<C: Clock>(&mut self, clock: &C, layout: &WrapLayout, viewport_rows: u16) {
        self.tick(clock, layout, viewport_rows);
        self.play_anchor = None;
        self.anchor_word_pos = self.word_pos;
    }

    pub fn resume<C: Clock>(&mut self, clock: &C) {
        self.anchor_word_pos = self.word_pos;
        self.play_anchor = Some(clock.now());
    }

    pub fn toggle<C: Clock>(&mut self, clock: &C, layout: &WrapLayout, viewport_rows: u16) {
        if self.is_playing() {
            self.pause(clock, layout, viewport_rows);
        } else {
            self.resume(clock);
        }
    }

    pub fn set_wpm<C: Clock>(
        &mut self,
        wpm: Wpm,
        clock: &C,
        layout: &WrapLayout,
        viewport_rows: u16,
    ) {
        self.tick(clock, layout, viewport_rows);
        let playing = self.is_playing();
        self.wpm = wpm;
        self.reseat(clock, playing);
    }

    pub fn nudge_wpm<C: Clock>(
        &mut self,
        delta: i32,
        clock: &C,
        layout: &WrapLayout,
        viewport_rows: u16,
    ) {
        self.set_wpm(self.wpm.nudge(delta), clock, layout, viewport_rows);
    }

    pub fn move_rows<C: Clock>(
        &mut self,
        delta_rows: f64,
        clock: &C,
        layout: &WrapLayout,
        viewport_rows: u16,
    ) {
        self.tick(clock, layout, viewport_rows);
        let rows = if self.word_pos >= layout.total_words - EPS {
            max_active_rows(layout)
        } else {
            layout.words_to_rows(self.word_pos)
        };
        let next = (rows + finite_or(delta_rows, 0.0)).clamp(0.0, max_active_rows(layout));
        self.word_pos = if next >= max_active_rows(layout) {
            layout.total_words
        } else {
            layout.rows_to_words(next)
        };
        self.clamp_to(layout, viewport_rows);
        self.reseat(clock, self.is_playing());
    }

    pub fn goto_line<C: Clock>(
        &mut self,
        line_idx: usize,
        clock: &C,
        layout: &WrapLayout,
        viewport_rows: u16,
    ) {
        let row = layout
            .rows
            .iter()
            .position(|visual| visual.line_idx >= line_idx)
            .unwrap_or(layout.total_rows);
        self.word_pos = layout.rows_to_words(row as f64);
        self.clamp_to(layout, viewport_rows);
        self.reseat(clock, self.is_playing());
    }

    pub fn home<C: Clock>(&mut self, clock: &C) {
        self.word_pos = 0.0;
        self.reseat(clock, self.is_playing());
    }

    pub fn end<C: Clock>(&mut self, clock: &C, layout: &WrapLayout, viewport_rows: u16) {
        let _ = viewport_rows;
        self.word_pos = max_word_pos(layout);
        self.reseat(clock, self.is_playing());
    }

    fn reseat<C: Clock>(&mut self, clock: &C, playing: bool) {
        self.anchor_word_pos = self.word_pos;
        self.play_anchor = playing.then(|| clock.now());
    }

    fn reseat_paused(&mut self) {
        self.anchor_word_pos = self.word_pos;
        self.play_anchor = None;
    }

    fn clamp_to(&mut self, layout: &WrapLayout, viewport_rows: u16) {
        let _ = viewport_rows;
        self.word_pos =
            finite_or(self.word_pos, self.anchor_word_pos).clamp(0.0, max_word_pos(layout));
        self.anchor_word_pos =
            finite_or(self.anchor_word_pos, self.word_pos).clamp(0.0, max_word_pos(layout));
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

fn max_active_rows(layout: &WrapLayout) -> f64 {
    layout.total_rows.saturating_sub(1) as f64
}

fn max_word_pos(layout: &WrapLayout) -> f64 {
    layout.total_words
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn wrap_ranges(text: &str, width: u16) -> Vec<(usize, usize)> {
    if text.is_empty() {
        return vec![(0, 0)];
    }

    let limit = width as usize;
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut used = 0usize;
        let mut end = start;
        let mut last_break = None;

        for (offset, ch) in text[start..].char_indices() {
            let idx = start + offset;
            let char_width = ch.width().unwrap_or(0).max(1);
            if used + char_width > limit && end > start {
                break;
            }
            used += char_width;
            end = idx + ch.len_utf8();
            if ch.is_whitespace() {
                last_break = Some(end);
            }
            if used >= limit {
                break;
            }
        }

        if end <= start {
            end = text[start..]
                .chars()
                .next()
                .map(|ch| start + ch.len_utf8())
                .unwrap_or(text.len());
        } else if end < text.len() {
            if let Some(break_at) = last_break.filter(|&break_at| break_at > start) {
                end = break_at;
            }
        }

        let trimmed_end = text[start..end].trim_end_matches(char::is_whitespace).len() + start;
        ranges.push((start, trimmed_end.max(start)));
        start = end;
        while start < text.len() {
            let Some(ch) = text[start..].chars().next() else {
                break;
            };
            if !ch.is_whitespace() {
                break;
            }
            start += ch.len_utf8();
        }
    }

    if ranges.is_empty() {
        vec![(0, 0)]
    } else {
        ranges
    }
}
