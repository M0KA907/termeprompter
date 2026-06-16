//! Pure rendering for Ratatui.

use crate::document::{Document, LineKind};
use crate::importer::{ImportKind, ImportMenu};
use crate::mirror::mirror_row;
use crate::scroll::{Wpm, WrapLayout};
use crate::theme::{Theme, Token};
use crate::timing::{fmt_clock, Estimate};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use serde::Deserialize;
use std::str::FromStr;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutKind {
    Prompt,
    Rehearsal,
    #[default]
    Horizontal,
    Trainer,
    Minimal,
}

impl FromStr for LayoutKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "prompt" => Ok(Self::Prompt),
            "rehearsal" => Ok(Self::Rehearsal),
            "horizontal" | "ticker" => Ok(Self::Horizontal),
            "trainer" | "speedread" | "speedreader" => Ok(Self::Trainer),
            "minimal" => Ok(Self::Minimal),
            other => Err(format!("unknown layout `{other}`")),
        }
    }
}

impl LayoutKind {
    pub fn cycle(self) -> Self {
        match self {
            Self::Horizontal => Self::Trainer,
            Self::Trainer => Self::Prompt,
            Self::Prompt => Self::Rehearsal,
            Self::Rehearsal => Self::Minimal,
            Self::Minimal => Self::Horizontal,
        }
    }
}

pub struct RenderCtx<'a> {
    pub doc: &'a Document,
    pub layout: &'a WrapLayout,
    pub scroll_rows: f64,
    pub wpm: Wpm,
    pub playing: bool,
    pub at_end: bool,
    pub est: Estimate,
    pub theme: &'a Theme,
    pub mirror: bool,
    pub layout_kind: LayoutKind,
    pub show_help: bool,
    pub import_menu: Option<&'a ImportMenu>,
    pub presentation: Option<PresentationView>,
    pub use_ascii: bool,
}

pub struct PresentationView {
    pub source_name: String,
    pub slide: usize,
    pub slide_count: usize,
}

pub fn content_dims(area: Rect, layout_kind: LayoutKind, _use_ascii: bool) -> (u16, u16) {
    let effective = if area.width < 20 || area.height < 6 {
        LayoutKind::Minimal
    } else {
        layout_kind
    };
    let status_rows = status_height(area.height, effective);
    let viewport_rows = area.height.saturating_sub(status_rows);
    let content_width = if matches!(effective, LayoutKind::Horizontal | LayoutKind::Trainer) {
        u16::MAX
    } else {
        area.width.clamp(1, 100)
    };
    (content_width, viewport_rows)
}

pub fn presentation_status_height(area_height: u16) -> u16 {
    status_height(area_height, LayoutKind::Prompt)
}

pub fn draw(f: &mut Frame, ctx: &RenderCtx) {
    let area = f.area();
    f.render_widget(Block::default().style(ctx.theme.style(Token::Bg)), area);

    if let Some(menu) = ctx.import_menu {
        draw_import(f, area, ctx, menu);
        return;
    }

    if ctx.presentation.is_some() {
        draw_presentation(f, area, ctx);
        return;
    }

    let effective = if area.width < 20 || area.height < 6 {
        LayoutKind::Minimal
    } else {
        ctx.layout_kind
    };

    match effective {
        LayoutKind::Minimal => draw_minimal(f, area, ctx),
        LayoutKind::Prompt => draw_prompt(f, area, ctx),
        LayoutKind::Rehearsal => draw_rehearsal(f, area, ctx),
        LayoutKind::Horizontal => draw_horizontal(f, area, ctx),
        LayoutKind::Trainer => draw_trainer(f, area, ctx),
    }
}

fn draw_presentation(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    let status_rows = presentation_status_height(area.height);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_rows)])
        .split(area);
    f.render_widget(
        Block::default().style(ctx.theme.style(Token::Bg)),
        vertical[0],
    );
    if status_rows > 0 {
        draw_status(f, vertical[1], ctx);
    }
}

fn status_height(area_height: u16, effective: LayoutKind) -> u16 {
    if area_height < 2 {
        0
    } else if matches!(effective, LayoutKind::Minimal) || area_height < 8 {
        1
    } else {
        2
    }
}

fn draw_minimal(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    let status_rows = status_height(area.height, LayoutKind::Minimal);
    let chunks = if status_rows > 0 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(status_rows)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    };
    draw_text(f, chunks[0], ctx, false, false);
    if status_rows > 0 {
        draw_status(f, chunks[1], ctx);
    }
}

fn draw_prompt(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    let status_rows = status_height(area.height, LayoutKind::Prompt);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_rows)])
        .split(area);
    let width = area.width.min(100);
    let gutters = area.width.saturating_sub(width) / 2;
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(gutters),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[0]);
    draw_text(f, horizontal[1], ctx, false, true);
    if status_rows > 0 {
        draw_status(f, vertical[1], ctx);
    }
}

fn draw_rehearsal(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    let status_rows = status_height(area.height, LayoutKind::Rehearsal);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_rows)])
        .split(area);
    draw_text(f, vertical[0], ctx, true, true);
    if status_rows > 0 {
        draw_status(f, vertical[1], ctx);
    }
}

fn draw_horizontal(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    let status_rows = status_height(area.height, LayoutKind::Horizontal);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_rows)])
        .split(area);
    draw_horizontal_text(f, vertical[0], ctx);
    if status_rows > 0 {
        draw_status(f, vertical[1], ctx);
    }
}

fn draw_trainer(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    let status_rows = status_height(area.height, LayoutKind::Trainer);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_rows)])
        .split(area);
    draw_trainer_text(f, vertical[0], ctx);
    if status_rows > 0 {
        draw_status(f, vertical[1], ctx);
    }
}

fn draw_text(f: &mut Frame, area: Rect, ctx: &RenderCtx, bordered: bool, ribbon: bool) {
    let inner = if bordered {
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(ctx.theme.style(Token::Dim));
        let inner = block.inner(area);
        f.render_widget(block, area);
        inner
    } else {
        area
    };

    if ctx.layout.total_rows == 0 {
        let msg = Paragraph::new("No script loaded")
            .alignment(Alignment::Center)
            .style(ctx.theme.style(Token::Dim));
        f.render_widget(msg, inner);
        return;
    }
    if inner.height == 0 {
        return;
    }

    let active = ctx
        .scroll_rows
        .floor()
        .clamp(0.0, ctx.layout.total_rows.saturating_sub(1) as f64) as usize;
    let visible_rows = inner.height as usize;
    let guide_row = ribbon.then_some(if inner.height > 2 {
        inner.height as usize / 3
    } else {
        0
    });
    let top = if ctx.layout.total_rows <= visible_rows {
        0
    } else if let Some(guide) = guide_row {
        active.saturating_sub(guide)
    } else {
        active.min(ctx.layout.total_rows.saturating_sub(visible_rows))
    };
    let pad_before = if ctx.layout.total_rows <= visible_rows {
        0
    } else {
        guide_row
            .map(|guide| guide.saturating_sub(active))
            .unwrap_or(0)
            .min(visible_rows)
    };
    let bottom = top
        .saturating_add(visible_rows.saturating_sub(pad_before))
        .min(ctx.layout.total_rows);
    let mut lines = (0..pad_before)
        .map(|_| {
            TextLine::from(Span::styled(
                " ".repeat(inner.width as usize),
                ctx.theme.style(Token::Bg),
            ))
        })
        .collect::<Vec<_>>();
    lines.extend(
        ctx.layout.rows[top..bottom]
            .iter()
            .enumerate()
            .map(|(offset, row)| {
                let source = &ctx.doc.lines[row.line_idx].text;
                let text = source.get(row.byte_range.0..row.byte_range.1).unwrap_or("");
                let mut display = if ctx.mirror {
                    mirror_row(text, inner.width)
                } else {
                    text.to_string()
                };
                let highlighted = ribbon && top + offset == active;
                if highlighted {
                    display = pad_to_width(display, inner.width);
                }
                TextLine::from(Span::styled(
                    display,
                    if highlighted {
                        style_for(ctx, row.kind)
                            .fg(ctx.theme.color(Token::Bg))
                            .bg(ctx.theme.color(Token::Cue))
                    } else if ribbon {
                        ctx.theme.style(Token::Dim)
                    } else {
                        style_for(ctx, row.kind)
                    },
                ))
            }),
    );

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(ctx.theme.style(Token::Fg));
    f.render_widget(para, inner);
}

fn draw_horizontal_text(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    if ctx.layout.total_rows == 0 {
        let msg = Paragraph::new("No script loaded")
            .alignment(Alignment::Center)
            .style(ctx.theme.style(Token::Dim));
        f.render_widget(msg, area);
        return;
    }
    if area.height == 0 || area.width == 0 {
        return;
    }

    let mut lines = Vec::new();
    let center = area.height as usize / 2;
    for row in 0..area.height as usize {
        let text = if row == center {
            tape_line(ctx, area.width)
        } else if row + 1 == center || row == center + 1 {
            TextLine::from(Span::styled(
                line_window("|", 0, area.width as usize),
                ctx.theme.style(Token::Cue),
            ))
        } else {
            TextLine::from(Span::styled(
                " ".repeat(area.width as usize),
                ctx.theme.style(Token::Bg),
            ))
        };
        lines.push(text);
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn tape_line(ctx: &RenderCtx, width: u16) -> TextLine<'static> {
    let width = width as usize;
    let active = active_row_idx(ctx);
    let Some(row) = ctx.layout.rows.get(active) else {
        return TextLine::from("");
    };
    let active_line_idx = row.line_idx;
    let active_text = ctx
        .doc
        .lines
        .get(active_line_idx)
        .map(|line| line.text.as_str())
        .unwrap_or("");
    let prev_text = active_line_idx
        .checked_sub(1)
        .and_then(|idx| ctx.doc.lines.get(idx))
        .map(|line| line.text.as_str())
        .unwrap_or("");
    let next_text = ctx
        .doc
        .lines
        .get(active_line_idx + 1)
        .map(|line| line.text.as_str())
        .unwrap_or("");
    let ranges = word_ranges(active_text);
    let frac = (ctx.scroll_rows - active as f64).clamp(0.0, 1.0);
    let word_idx = if ranges.is_empty() {
        0
    } else {
        ((frac * ranges.len() as f64).floor() as usize).min(ranges.len() - 1)
    };
    let (word_start, word_end) = ranges
        .get(word_idx)
        .copied()
        .unwrap_or((0, active_text.len()));
    let edge_pad = " ".repeat(width / 2);
    let prev_context = context_prefix(prev_text);
    let prev_sep = if prev_context.is_empty() { "" } else { "   " };
    let tape_prefix = format!("{edge_pad}{prev_context}{prev_sep}");
    let before = format!(
        "{}{}",
        tape_prefix,
        active_text.get(..word_start).unwrap_or("")
    );
    let focus = active_text.get(word_start..word_end).unwrap_or(active_text);
    let next_context = context_suffix(next_text);
    let next_sep = if next_context.is_empty() { "" } else { "   " };
    let after = format!(
        "{}{}{}{}",
        active_text.get(word_end..).unwrap_or(""),
        next_sep,
        next_context,
        edge_pad
    );
    let focus_center = tape_word_center(&tape_prefix, active_text, word_start, word_end);
    let offset = (focus_center - width as f64 / 2.0).round().max(0.0) as usize;
    let focus_style = ctx
        .theme
        .style(Token::Cue)
        .fg(ctx.theme.color(Token::Bg))
        .bg(ctx.theme.color(Token::Cue));
    let dim_style = ctx.theme.style(Token::Dim);
    let mut line = clipped_spans(
        &[
            (before, dim_style),
            (focus.to_string(), focus_style),
            (after, dim_style),
        ],
        offset,
        width,
        ctx.theme.style(Token::Bg),
    );
    if ctx.mirror {
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        line = TextLine::from(Span::styled(
            mirror_row(&text, width as u16),
            ctx.theme.style(Token::Dim),
        ));
    }
    line
}

fn word_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(word_start) = start.take() {
                ranges.push((word_start, idx));
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(word_start) = start {
        ranges.push((word_start, text.len()));
    }
    ranges
}

fn tape_word_center(prefix: &str, active_text: &str, start: usize, end: usize) -> f64 {
    let before = active_text.get(..start).unwrap_or("");
    let word = active_text.get(start..end).unwrap_or("");
    UnicodeWidthStr::width(prefix) as f64
        + UnicodeWidthStr::width(before) as f64
        + UnicodeWidthStr::width(word) as f64 / 2.0
}

fn context_prefix(text: &str) -> String {
    text.split_whitespace()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ")
}

fn context_suffix(text: &str) -> String {
    text.split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ")
}

fn clipped_spans(
    segments: &[(String, Style)],
    offset: usize,
    width: usize,
    fill_style: Style,
) -> TextLine<'static> {
    if width == 0 {
        return TextLine::from("");
    }
    let mut spans = Vec::new();
    let mut seen = 0usize;
    let mut used = 0usize;

    'segments: for (text, style) in segments {
        let mut chunk = String::new();
        for ch in text.chars() {
            let ch_width = ch.width().unwrap_or(0).max(1);
            if seen + ch_width <= offset {
                seen += ch_width;
                continue;
            }
            if seen < offset {
                seen += ch_width;
                continue;
            }
            if used + ch_width > width {
                break 'segments;
            }
            chunk.push(ch);
            seen += ch_width;
            used += ch_width;
        }
        if !chunk.is_empty() {
            spans.push(Span::styled(chunk, *style));
        }
    }

    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), fill_style));
    }
    TextLine::from(spans)
}

fn draw_trainer_text(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    if ctx.layout.total_rows == 0 {
        let msg = Paragraph::new("No script loaded")
            .alignment(Alignment::Center)
            .style(ctx.theme.style(Token::Dim));
        f.render_widget(msg, area);
        return;
    }
    if area.height == 0 || area.width == 0 {
        return;
    }

    let active = active_row_idx(ctx);
    let Some(row) = ctx.layout.rows.get(active) else {
        return;
    };
    let source = &ctx.doc.lines[row.line_idx].text;
    let text = source.get(row.byte_range.0..row.byte_range.1).unwrap_or("");
    let words = text.split_whitespace().collect::<Vec<_>>();
    let chunk = trainer_chunk_size(ctx.wpm.get());
    let frac = (ctx.scroll_rows - active as f64).clamp(0.0, 1.0);
    let word_idx = if words.is_empty() {
        0
    } else {
        ((frac * words.len() as f64).floor() as usize).min(words.len() - 1)
    };
    let chunk_start = (word_idx / chunk) * chunk;
    let chunk_end = (chunk_start + chunk).min(words.len());
    let phrase = if words.is_empty() {
        text.trim().to_string()
    } else {
        words[chunk_start..chunk_end].join(" ")
    };

    let prev = trainer_context(&words, chunk_start.saturating_sub(chunk), chunk_start);
    let next = trainer_context(&words, chunk_end, (chunk_end + chunk).min(words.len()));
    let center = area.height as usize / 2;
    let progress = if words.is_empty() {
        1.0
    } else {
        (word_idx + 1) as f64 / words.len() as f64
    };
    let pace = trainer_pace_line(ctx, chunk, progress, area.width);
    let mut lines = Vec::with_capacity(area.height as usize);

    for row_idx in 0..area.height as usize {
        let line = if row_idx == center.saturating_sub(4) {
            pace.clone()
        } else if row_idx + 2 == center {
            TextLine::from(Span::styled(
                line_window(&prev, 0, area.width as usize),
                ctx.theme.style(Token::Dim),
            ))
        } else if row_idx + 1 == center {
            TextLine::from(Span::styled(
                line_window("|", 0, area.width as usize),
                ctx.theme.style(Token::Cue),
            ))
        } else if row_idx == center {
            TextLine::from(Span::styled(
                line_window(&phrase, 0, area.width as usize),
                ctx.theme
                    .style(Token::Cue)
                    .fg(ctx.theme.color(Token::Bg))
                    .bg(ctx.theme.color(Token::Cue)),
            ))
        } else if row_idx == center + 1 {
            TextLine::from(Span::styled(
                line_window("|", 0, area.width as usize),
                ctx.theme.style(Token::Cue),
            ))
        } else if row_idx == center + 2 {
            TextLine::from(Span::styled(
                line_window(&next, 0, area.width as usize),
                ctx.theme.style(Token::Dim),
            ))
        } else {
            TextLine::from(Span::styled(
                " ".repeat(area.width as usize),
                ctx.theme.style(Token::Bg),
            ))
        };
        lines.push(line);
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn trainer_pace_line(
    ctx: &RenderCtx,
    chunk: usize,
    progress: f64,
    width: u16,
) -> TextLine<'static> {
    let width = width as usize;
    let rail_width = width.saturating_sub(34).clamp(8, 28);
    let rail = progress_rail(progress, rail_width, ctx.use_ascii);
    let text = format!("TRAIN {:>4} wpm  chunk {}  {}", ctx.wpm.get(), chunk, rail);
    TextLine::from(Span::styled(
        line_window(&text, 0, width),
        ctx.theme.style(Token::Accent).add_modifier(Modifier::BOLD),
    ))
}

fn trainer_chunk_size(wpm: u16) -> usize {
    match wpm {
        0..=159 => 1,
        160..=219 => 2,
        _ => 3,
    }
}

fn trainer_context(words: &[&str], start: usize, end: usize) -> String {
    if start >= end || start >= words.len() {
        String::new()
    } else {
        words[start..end.min(words.len())].join(" ")
    }
}

fn pad_to_width(mut text: String, width: u16) -> String {
    let target = width as usize;
    let cells = UnicodeWidthStr::width(text.as_str());
    if cells < target {
        text.push_str(&" ".repeat(target - cells));
    }
    text
}

fn draw_status(f: &mut Frame, area: Rect, ctx: &RenderCtx) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    if let Some(presentation) = &ctx.presentation {
        draw_presentation_status(f, area, ctx, presentation);
        return;
    }

    let state = if ctx.at_end {
        "end"
    } else if ctx.playing {
        "play"
    } else {
        "pause"
    };
    let mirror = if ctx.mirror { "mirror" } else { "normal" };
    let progress = (ctx.est.progress * 100.0).round() as u16;
    let layout = match ctx.layout_kind {
        LayoutKind::Prompt => "prompt",
        LayoutKind::Rehearsal => "rehearsal",
        LayoutKind::Horizontal => "horizontal",
        LayoutKind::Trainer => "trainer",
        LayoutKind::Minimal => "minimal",
    };
    let info_line = if area.width >= 90 {
        status_line(
            ctx,
            &[
                ("state", state.to_string()),
                ("wpm", ctx.wpm.get().to_string()),
                ("layout", layout.to_string()),
                ("read", progress_bar(ctx.est.progress, 18, ctx.use_ascii)),
                ("left", fmt_clock(ctx.est.remaining)),
                ("elapsed", fmt_clock(ctx.est.elapsed)),
                ("progress", format!("{progress:>3}%")),
                ("mode", mirror.to_string()),
            ],
            area.width,
        )
    } else {
        status_line(
            ctx,
            &[
                ("state", state.to_string()),
                ("wpm", ctx.wpm.get().to_string()),
                ("layout", layout.to_string()),
                ("read", progress_bar(ctx.est.progress, 10, ctx.use_ascii)),
                ("progress", format!("{progress:>3}%")),
            ],
            area.width,
        )
    };
    let style = ctx
        .theme
        .style(Token::StatusFg)
        .bg(ctx.theme.color(Token::StatusBg));

    if area.height == 1 {
        let merged = format!(
            "{} | {}",
            status_plain(ctx, state, layout, progress),
            controls_text(ctx)
        );
        f.render_widget(
            Paragraph::new(scroll_text(&merged, area.width, ctx)).style(style),
            area,
        );
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        f.render_widget(Paragraph::new(info_line).style(style), rows[0]);
        f.render_widget(
            Paragraph::new(control_line(ctx, area.width)).style(style),
            rows[1],
        );
    }
}

fn draw_presentation_status(
    f: &mut Frame,
    area: Rect,
    ctx: &RenderCtx,
    presentation: &PresentationView,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let style = ctx
        .theme
        .style(Token::StatusFg)
        .bg(ctx.theme.color(Token::StatusBg));
    let progress = format!("{}/{}", presentation.slide, presentation.slide_count);
    let info_line = status_line(
        ctx,
        &[
            ("mode", "slides".to_string()),
            ("file", presentation.source_name.clone()),
            ("slide", progress),
        ],
        area.width,
    );

    if area.height == 1 {
        let merged = format!(
            "slides | {} | slide {}/{} | {}",
            presentation.source_name,
            presentation.slide,
            presentation.slide_count,
            controls_text(ctx)
        );
        f.render_widget(
            Paragraph::new(scroll_text(&merged, area.width, ctx)).style(style),
            area,
        );
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        f.render_widget(Paragraph::new(info_line).style(style), rows[0]);
        f.render_widget(
            Paragraph::new(control_line(ctx, area.width)).style(style),
            rows[1],
        );
    }
}

fn status_plain(ctx: &RenderCtx, state: &str, layout: &str, progress: u16) -> String {
    format!(
        "{state} | {} wpm | {layout} | {} | {:>3}%",
        ctx.wpm.get(),
        progress_bar(ctx.est.progress, 10, ctx.use_ascii),
        progress
    )
}

fn status_line(ctx: &RenderCtx, items: &[(&str, String)], width: u16) -> TextLine<'static> {
    let mut spans = Vec::new();
    spans.push(Span::styled(
        " TP ",
        ctx.theme
            .style(Token::Cue)
            .fg(ctx.theme.color(Token::Bg))
            .bg(ctx.theme.color(Token::Cue))
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(" ", ctx.theme.style(Token::StatusFg)));

    for (idx, (label, value)) in items.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" | ", ctx.theme.style(Token::Dim)));
        }
        spans.push(Span::styled(
            format!("{label} "),
            ctx.theme
                .style(Token::Dim)
                .bg(ctx.theme.color(Token::StatusBg)),
        ));
        spans.push(Span::styled(
            value.clone(),
            ctx.theme
                .style(Token::StatusFg)
                .bg(ctx.theme.color(Token::StatusBg))
                .add_modifier(Modifier::BOLD),
        ));
    }

    pad_spans(spans, width as usize, ctx.theme.style(Token::StatusFg))
}

fn control_line(ctx: &RenderCtx, width: u16) -> TextLine<'static> {
    let text = scroll_text(controls_text(ctx), width, ctx);
    let prefix = if ctx.use_ascii { "CTRL " } else { "CTRL ▸ " };
    let prefix_width = UnicodeWidthStr::width(prefix);
    let body_width = (width as usize).saturating_sub(prefix_width);
    let body = cell_window(&text, 0, body_width);
    let spans = vec![
        Span::styled(
            prefix.to_string(),
            ctx.theme
                .style(Token::Accent)
                .bg(ctx.theme.color(Token::StatusBg))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            body,
            ctx.theme
                .style(Token::Dim)
                .bg(ctx.theme.color(Token::StatusBg)),
        ),
    ];
    pad_spans(spans, width as usize, ctx.theme.style(Token::StatusFg))
}

fn controls_text(ctx: &RenderCtx) -> &'static str {
    if ctx.show_help {
        "Help: Esc/?/h close help | q or Ctrl-C quit"
    } else if ctx.import_menu.is_some() {
        "Import: Enter open/import | Backspace/Left/h parent | Up/k select up | Down/j select down | Esc/i close | q or Ctrl-C quit"
    } else if ctx.presentation.is_some() {
        "Slides: Space/Enter/Right/Down/PageDown next | Left/Up/PageUp previous | Home first | End last | i import | ?/h help | q/Esc quit"
    } else {
        "Keys: Space/p/Enter play | f/+/= faster | s/-/_ slower | Up/k row up | Down/j row down | PageUp/u page up | PageDown/d page down | Home/g start | End/G end | 1-9 cues | i import | m mirror | l layout | [ previous theme | ]/t next theme | ?/h help | q/Esc/Ctrl-C quit"
    }
}

fn fit_status(text: &str, width: u16) -> String {
    cell_window(text, 0, width as usize)
}

fn scroll_text(text: &str, width: u16, ctx: &RenderCtx) -> String {
    let width = width as usize;
    if UnicodeWidthStr::width(text) <= width {
        return pad_to_cells(text.to_string(), width);
    }
    let text_width = UnicodeWidthStr::width(text);
    let max_offset = text_width.saturating_sub(width);
    if max_offset == 0 {
        return cell_window(text, 0, width);
    }
    let period = (max_offset * 2).max(1);
    let phase = ((ctx.est.elapsed.as_secs_f64() * 5.0).floor() as usize) % period;
    let offset = if phase <= max_offset {
        phase
    } else {
        period - phase
    };
    cell_window(text, offset, width)
}

fn draw_import(f: &mut Frame, area: Rect, ctx: &RenderCtx, menu: &ImportMenu) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" OPEN / IMPORT ")
        .border_style(ctx.theme.style(Token::Accent))
        .style(ctx.theme.style(Token::BgDim));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let visible_items = inner.height.saturating_sub(4).max(1) as usize;
    let total_items = menu.entries.len() + 1;
    let start = menu
        .selected
        .saturating_sub(visible_items.saturating_sub(1))
        .min(total_items.saturating_sub(visible_items));
    let end = (start + visible_items).min(total_items);

    let controls = "ENTER open/import  BACKSPACE/LEFT/H parent  UP/K select up  DOWN/J select down  ESC/I close  Q/CTRL-C quit";
    let mut lines = vec![
        TextLine::from(Span::styled(
            fit_status(&format!("Folder {}", menu.cwd.display()), inner.width),
            ctx.theme.style(Token::Heading).add_modifier(Modifier::BOLD),
        )),
        TextLine::from(Span::styled(
            scroll_text(controls, inner.width, ctx),
            ctx.theme.style(Token::Accent),
        )),
        TextLine::from(""),
    ];

    for idx in start..end {
        let selected = idx == menu.selected;
        let text = if idx == 0 {
            "folder  Import this folder as text script".to_string()
        } else {
            let entry = &menu.entries[idx - 1];
            format!("{}  {}", import_label(entry.kind), entry.name)
        };
        let marker = if selected { "> " } else { "  " };
        let style = if selected {
            ctx.theme
                .style(Token::Accent)
                .bg(ctx.theme.color(Token::StatusBg))
                .add_modifier(Modifier::BOLD)
        } else {
            ctx.theme.style(Token::Fg)
        };
        lines.push(TextLine::from(Span::styled(
            fit_status(&format!("{marker}{text}"), inner.width),
            style,
        )));
    }

    if let Some(message) = &menu.message {
        lines.push(TextLine::from(""));
        lines.push(TextLine::from(Span::styled(
            message.clone(),
            ctx.theme.style(Token::Cue),
        )));
    }

    f.render_widget(
        Paragraph::new(lines).style(ctx.theme.style(Token::Fg)),
        inner,
    );
}

fn import_label(kind: ImportKind) -> &'static str {
    match kind {
        ImportKind::Folder => "folder",
        ImportKind::Text => "text",
        ImportKind::PowerPoint => "powerpoint",
        ImportKind::Slideshow => "slideshow",
    }
}

fn style_for(ctx: &RenderCtx, kind: LineKind) -> ratatui::style::Style {
    match kind {
        LineKind::Heading(_) => ctx.theme.style(Token::Heading),
        LineKind::Cue => ctx.theme.style(Token::Cue),
        LineKind::Rule => ctx.theme.style(Token::Dim),
        LineKind::Blank => ctx.theme.style(Token::Dim),
        LineKind::Body => ctx.theme.style(Token::Fg),
    }
}

fn active_row_idx(ctx: &RenderCtx) -> usize {
    ctx.scroll_rows
        .floor()
        .clamp(0.0, ctx.layout.total_rows.saturating_sub(1) as f64) as usize
}

fn cell_window(text: &str, offset: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut seen = 0usize;
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0).max(1);
        if seen + ch_width <= offset {
            seen += ch_width;
            continue;
        }
        if used + ch_width > width {
            break;
        }
        out.push(ch);
        used += ch_width;
        seen += ch_width;
    }
    pad_to_cells(out, width)
}

fn line_window(text: &str, offset: usize, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    if text_width >= width {
        return cell_window(text, offset, width);
    }
    let left = (width - text_width) / 2;
    let mut out = String::with_capacity(width);
    out.push_str(&" ".repeat(left));
    out.push_str(text);
    pad_to_cells(out, width)
}

fn progress_rail(progress: f64, width: usize, ascii: bool) -> String {
    let width = width.max(1);
    let filled = (progress.clamp(0.0, 1.0) * width as f64).round() as usize;
    let (on, off) = if ascii { ("=", "-") } else { ("━", "·") };
    let mut out = String::with_capacity(width);
    for idx in 0..width {
        out.push_str(if idx < filled { on } else { off });
    }
    out
}

fn progress_bar(progress: f64, width: usize, ascii: bool) -> String {
    let rail = progress_rail(progress, width, ascii);
    if ascii {
        format!("[{rail}]")
    } else {
        format!("▐{rail}▌")
    }
}

fn pad_spans(mut spans: Vec<Span<'static>>, width: usize, style: Style) -> TextLine<'static> {
    let used = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum::<usize>();
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), style));
    }
    TextLine::from(spans)
}

fn pad_to_cells(mut text: String, width: usize) -> String {
    let cells = UnicodeWidthStr::width(text.as_str());
    if cells < width {
        text.push_str(&" ".repeat(width - cells));
    }
    text
}
