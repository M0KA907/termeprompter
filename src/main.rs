use anyhow::Context;
use termeprompter::app::{run, AppState};
use termeprompter::cli::Cli;
use termeprompter::config::Config;
use termeprompter::importer;
use termeprompter::importer::ImportMenu;
use termeprompter::parser;
use termeprompter::presentation::{
    clear_graphics, is_presentation_path, kitty_graphics_available, Presentation,
};
use termeprompter::terminal::{install_panic_hook, TerminalGuard};
use termeprompter::timing::SystemClock;

const DEMO: &str = r#"# Orbital Product Briefing

[cue:cold-open]
Black screen.

One line of light appears.

The camera finds a presenter standing beside a terminal window that looks too calm for the amount of work it is doing.

Good evening.

This is termeprompter: a terminal-native teleprompter for launches, livestreams, tutorials, speeches, and late-night release notes.

It is local. It is offline. It does not ask for an account. It does not report back to anything.

[cue:pace]
The words move upward like a measured crawl.

The ribbon is the read line.

Keep your eyes there.

Let the script come to you.

If the room gets loud, slow the pace.

If the take is clean, speed it up.

Your hands stay on the keyboard. Your attention stays on the sentence.

---

[cue:demo-beats]
Space pauses the crawl.

Plus and minus change words per minute.

J and K nudge the script by one row.

Page Up and Page Down move by a larger beat.

M flips mirror mode for glass or camera rigs.

L cycles layouts when you want less chrome.

Left and right brackets cycle themes when the room lighting changes.

Question mark opens help.

[cue:resize]
Now resize the terminal.

The text reflows, but the reading position stays attached to the same word.

That is the trick: the cursor lives on the word axis, not on fragile screen rows.

The terminal can change shape.

The script does not lose its place.

---

[cue:close]
Final beat.

The ribbon keeps moving.

The side panel keeps time.

The text remains the star.

Press Q when the take is finished.
"#;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse_args()?;
    let demo_mode = cli.demo;
    let open_import = !demo_mode && cli.path.is_none();
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
                cli.path,
                kitty_graphics_available()
            ),
        );
    }

    let mut startup_presentation = None;
    let mut doc = match (demo_mode, cli.path) {
        (true, _) => parser::parse(DEMO),
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
