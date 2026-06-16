use anyhow::Context;
use termeprompter::app::{run, AppState};
use termeprompter::cli::Cli;
use termeprompter::config::Config;
use termeprompter::demo;
use termeprompter::importer;
use termeprompter::importer::ImportMenu;
use termeprompter::parser;
use termeprompter::presentation::{
    clear_graphics, is_presentation_path, kitty_graphics_available, Presentation,
};
use termeprompter::terminal::{install_panic_hook, TerminalGuard};
use termeprompter::timing::SystemClock;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse_args()?;
    let demo_mode = cli.demo;
    let slides_demo_path = if cli.slides_demo {
        Some(demo::write_slides_demo()?)
    } else {
        None
    };
    let input_path = slides_demo_path.clone().or(cli.path);
    let open_import = !demo_mode && input_path.is_none();
    let mut cfg = Config::load_default()?;
    if let Some(wpm) = cli.wpm {
        cfg.wpm = wpm;
    }
    if let Some(theme) = cli.theme {
        cfg.theme = theme;
    }
    if let Some(layout) = cli.layout {
        cfg.layout = layout;
    }
    cfg.mirror |= cli.mirror;
    cfg.ascii |= cli.ascii || std::env::var_os("NO_UNICODE").is_some();

    let dbg = std::env::var_os("TERMEPROMPTER_DEBUG").is_some();
    if dbg {
        let _ = std::fs::write(
            "/tmp/tp-debug.log",
            format!(
                "path={:?} demo={demo_mode} open_import={open_import} kitty={}\n",
                input_path,
                kitty_graphics_available()
            ),
        );
    }

    let mut startup_presentation = None;
    let mut doc = match (demo_mode, input_path) {
        (true, _) => parser::parse(demo::TEXT),
        (false, None) => parser::parse(""),
        (false, Some(path)) if is_presentation_path(&path) && kitty_graphics_available() => {
            match Presentation::open(&path) {
                Ok(presentation) => {
                    startup_presentation = Some(presentation);
                    parser::parse("")
                }
                // Rich render failed; fall back to text extraction.
                Err(_) => importer::load_import(&path)?,
            }
        }
        (false, Some(path)) => importer::load_import(&path)?,
    };
    if let Some(path) = &slides_demo_path {
        let _ = std::fs::remove_file(path);
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir(dir);
        }
    }
    if demo_mode {
        doc.source_path = None;
    }

    install_panic_hook();
    let mut guard = TerminalGuard::enter().context("failed to enter terminal mode")?;
    // Wipe any stale Kitty image left by a previous/crashed run so it can't
    // show through the import browser.
    let _ = clear_graphics(guard.terminal().backend_mut());
    let clock = SystemClock;
    let mut app = AppState::new(doc, &cfg);
    app.presentation = startup_presentation;
    if demo_mode {
        app.scroll.resume(&clock);
    } else if open_import {
        app.import_menu = Some(ImportMenu::open());
        app.require_import = true;
    }
    if dbg {
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("/tmp/tp-debug.log")
        {
            let _ = writeln!(
                f,
                "presentation={} import_menu={} require_import={}",
                app.presentation.is_some(),
                app.import_menu.is_some(),
                app.require_import,
            );
        }
    }
    run(app, &clock, &mut guard)
}
