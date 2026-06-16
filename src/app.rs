//! App state and main loop.

use crate::config::Config;
use crate::document::Document;
use crate::importer::{load_import, ImportKind, ImportMenu};
use crate::input::{map_key, Action, InputCtx};
use crate::presentation::{is_presentation_path, kitty_graphics_available, Presentation};
use crate::render::{self, LayoutKind, RenderCtx};
use crate::scroll::{ScrollState, Wpm, WrapLayout};
use crate::terminal::TerminalGuard;
use crate::theme::Theme;
use crate::timing::{estimate, Clock};
use crossterm::event::{self, Event};
use ratatui::layout::Rect;
use std::time::Duration;

pub struct AppState {
    pub doc: Document,
    pub scroll: ScrollState,
    pub theme: Theme,
    pub layout_kind: LayoutKind,
    pub mirror: bool,
    pub show_help: bool,
    pub import_menu: Option<ImportMenu>,
    pub presentation: Option<Presentation>,
    pub require_import: bool,
    pub use_ascii: bool,
    pub should_quit: bool,
    pub area: (u16, u16),
    wrap: Option<WrapLayout>,
    elapsed: Duration,
}

impl AppState {
    pub fn new(doc: Document, cfg: &Config) -> Self {
        let mut scroll = ScrollState::new();
        scroll.set_wpm(
            Wpm::new(cfg.wpm),
            &crate::timing::SystemClock,
            &WrapLayout::build(&doc, 1),
            0,
        );
        Self {
            doc,
            scroll,
            theme: Theme::new(cfg.theme),
            layout_kind: cfg.layout,
            mirror: cfg.mirror,
            show_help: false,
            import_menu: None,
            presentation: None,
            require_import: false,
            use_ascii: cfg.ascii,
            should_quit: false,
            area: (0, 0),
            wrap: None,
            elapsed: Duration::ZERO,
        }
    }

    pub fn ensure_wrap(&mut self, content_width: u16) {
        let width = content_width.max(1);
        let needs_rebuild = self
            .wrap
            .as_ref()
            .map(|wrap| wrap.width != width || wrap.doc_version != self.doc.version)
            .unwrap_or(true);
        if needs_rebuild {
            self.wrap = Some(WrapLayout::build(&self.doc, width));
        }
    }

    pub fn apply<C: Clock>(&mut self, action: Action, clock: &C) {
        match action {
            Action::OpenImport => {
                self.show_help = false;
                self.import_menu = Some(ImportMenu::open());
                return;
            }
            Action::ImportClose => {
                if self.require_import {
                    self.should_quit = true;
                } else {
                    self.import_menu = None;
                }
                return;
            }
            Action::ImportUp => {
                if let Some(menu) = &mut self.import_menu {
                    menu.move_selection(-1);
                }
                return;
            }
            Action::ImportDown => {
                if let Some(menu) = &mut self.import_menu {
                    menu.move_selection(1);
                }
                return;
            }
            Action::ImportParent => {
                if let Some(menu) = &mut self.import_menu {
                    menu.parent();
                }
                return;
            }
            Action::ImportConfirm => {
                self.confirm_import(clock);
                return;
            }
            _ => {}
        }

        if self.presentation.is_some() {
            self.apply_presentation_action(action);
            return;
        }

        let (content_width, viewport_rows) = render::content_dims(
            Rect::new(0, 0, self.area.0, self.area.1),
            self.layout_kind,
            self.use_ascii,
        );
        self.ensure_wrap(content_width);
        let layout = self.wrap.as_ref().expect("wrap is ensured");

        match action {
            Action::Quit => self.should_quit = true,
            Action::TogglePlay => self.scroll.toggle(clock, layout, viewport_rows),
            Action::SpeedUp => self.scroll.nudge_wpm(5, clock, layout, viewport_rows),
            Action::SpeedDown => self.scroll.nudge_wpm(-5, clock, layout, viewport_rows),
            Action::LineUp => self.scroll.move_rows(-1.0, clock, layout, viewport_rows),
            Action::LineDown => self.scroll.move_rows(1.0, clock, layout, viewport_rows),
            Action::PageUp => {
                let page = viewport_rows.saturating_sub(2).max(1) as f64;
                self.scroll.move_rows(-page, clock, layout, viewport_rows);
            }
            Action::PageDown => {
                let page = viewport_rows.saturating_sub(2).max(1) as f64;
                self.scroll.move_rows(page, clock, layout, viewport_rows);
            }
            Action::Home => self.scroll.home(clock),
            Action::End => self.scroll.end(clock, layout, viewport_rows),
            Action::GotoCue(index) => {
                if let Some(cue) = self.doc.cues.get(index) {
                    self.scroll
                        .goto_line(cue.line_idx, clock, layout, viewport_rows);
                }
            }
            Action::ToggleMirror => self.mirror = !self.mirror,
            Action::CycleLayout => self.layout_kind = self.layout_kind.cycle(),
            Action::ThemePrev => self.theme = self.theme.previous(),
            Action::ThemeNext => self.theme = self.theme.next(),
            Action::ToggleHelp => {
                self.import_menu = None;
                self.presentation = None;
                self.show_help = !self.show_help;
            }
            Action::Reload => {}
            Action::Resize(w, h) => self.area = (w, h),
            Action::Tick => self.scroll.tick(clock, layout, viewport_rows),
            Action::OpenImport
            | Action::ImportConfirm
            | Action::ImportClose
            | Action::ImportParent
            | Action::ImportUp
            | Action::ImportDown => {}
            Action::None => {}
        }
    }

    fn confirm_import<C: Clock>(&mut self, clock: &C) {
        let target = {
            let Some(menu) = &mut self.import_menu else {
                return;
            };

            if menu.selected == 0 {
                menu.cwd.clone()
            } else {
                let Some(entry) = menu.entries.get(menu.selected - 1).cloned() else {
                    return;
                };
                if matches!(entry.kind, ImportKind::Folder) {
                    menu.cwd = entry.path;
                    menu.selected = 0;
                    menu.refresh();
                    return;
                }
                entry.path
            }
        };

        if is_presentation_path(&target) && kitty_graphics_available() {
            if let Ok(presentation) = Presentation::open(&target) {
                self.presentation = Some(presentation);
                self.import_menu = None;
                self.require_import = false;
                self.show_help = false;
                return;
            }
            // Rich render failed (e.g. LibreOffice busy); fall back to text import
            // so the deck still opens instead of stranding the browser.
        }

        match load_import(&target) {
            Ok(mut doc) => {
                doc.version = self.doc.version.saturating_add(1);
                self.doc = doc;
                self.presentation = None;
                self.wrap = None;
                self.scroll.home(clock);
                self.import_menu = None;
                self.require_import = false;
            }
            Err(err) => {
                if let Some(menu) = &mut self.import_menu {
                    menu.message = Some(err.to_string());
                }
            }
        }
    }

    fn apply_presentation_action(&mut self, action: Action) {
        let Some(presentation) = &mut self.presentation else {
            return;
        };

        match action {
            Action::Quit => self.should_quit = true,
            Action::OpenImport => {
                self.import_menu = Some(ImportMenu::open());
                self.show_help = false;
            }
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::LineUp | Action::PageUp | Action::SpeedDown => presentation.previous(),
            Action::LineDown
            | Action::PageDown
            | Action::TogglePlay
            | Action::SpeedUp
            | Action::ImportConfirm => presentation.next(),
            Action::Home => presentation.first(),
            Action::End => presentation.last(),
            Action::Resize(w, h) => self.area = (w, h),
            _ => {}
        }
    }

    pub fn render_ctx(&self, viewport_rows: u16) -> RenderCtx<'_> {
        let layout = self.wrap.as_ref().expect("wrap is ensured before render");
        let scroll_rows = self.scroll.scroll_rows(layout, viewport_rows);
        RenderCtx {
            doc: &self.doc,
            layout,
            scroll_rows,
            wpm: self.scroll.wpm(),
            playing: self.scroll.is_playing(),
            at_end: self.scroll.at_end(layout, viewport_rows),
            est: estimate(
                self.scroll.word_pos(),
                layout,
                self.scroll.wpm(),
                self.elapsed,
            ),
            theme: &self.theme,
            mirror: self.mirror,
            layout_kind: self.layout_kind,
            show_help: self.show_help,
            import_menu: self.import_menu.as_ref(),
            presentation: self
                .presentation
                .as_ref()
                .map(|presentation| render::PresentationView {
                    source_name: presentation.source_name(),
                    slide: presentation.current_slide(),
                    slide_count: presentation.slide_count(),
                }),
            use_ascii: self.use_ascii,
        }
    }
}

pub fn run<C: Clock>(
    mut app: AppState,
    clock: &C,
    guard: &mut TerminalGuard,
) -> anyhow::Result<()> {
    const FRAME: Duration = Duration::from_millis(33);
    let started_at = clock.now();
    let mut next_frame = clock.now() + FRAME;
    let mut resize_pending = None;

    loop {
        let timeout = next_frame.saturating_duration_since(clock.now());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => app.apply(
                    map_key(
                        key,
                        InputCtx {
                            help_open: app.show_help,
                            import_open: app.import_menu.is_some(),
                        },
                    ),
                    clock,
                ),
                Event::Resize(w, h) => resize_pending = Some((w, h)),
                _ => {}
            }
        }

        if clock.now() >= next_frame {
            if let Some((w, h)) = resize_pending.take() {
                app.apply(Action::Resize(w, h), clock);
            }
            app.elapsed = clock.now().saturating_duration_since(started_at);
            app.apply(Action::Tick, clock);
            guard.terminal().draw(|f| {
                let area = f.area();
                app.area = (area.width, area.height);
                let (content_width, viewport_rows) =
                    render::content_dims(area, app.layout_kind, app.use_ascii);
                app.ensure_wrap(content_width);
                let layout = app.wrap.as_ref().expect("wrap is ensured");
                app.scroll.tick(clock, layout, viewport_rows);
                let ctx = app.render_ctx(viewport_rows);
                render::draw(f, &ctx);
            })?;
            let area = Rect::new(0, 0, app.area.0, app.area.1);
            let status_rows = render::presentation_status_height(area.height);
            let image_area = Rect::new(0, 0, area.width, area.height.saturating_sub(status_rows));
            if app.import_menu.is_none() {
                if let Some(presentation) = &mut app.presentation {
                    presentation.draw(guard.terminal().backend_mut(), image_area)?;
                }
            } else {
                // Menu is open: wipe any slide image so it can't bleed under the
                // browser, even when no presentation is currently loaded.
                if let Some(presentation) = &mut app.presentation {
                    presentation.clear(guard.terminal().backend_mut())?;
                } else {
                    crate::presentation::clear_graphics(guard.terminal().backend_mut())?;
                }
            }
            next_frame += FRAME;
            if next_frame < clock.now() {
                next_frame = clock.now() + FRAME;
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
