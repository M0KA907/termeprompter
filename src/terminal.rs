//! Terminal raw-mode guard and panic restoration.

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout, Stdout};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

static RESTORED: AtomicBool = AtomicBool::new(true);
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

pub struct TerminalGuard {
    term: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        install_panic_hook();
        RESTORED.store(false, Ordering::SeqCst);
        if let Err(err) = enable_raw_mode() {
            RESTORED.store(true, Ordering::SeqCst);
            return Err(err);
        }

        let mut out = stdout();
        if let Err(err) = execute!(out, EnterAlternateScreen, Hide) {
            restore();
            return Err(err);
        }

        let backend = CrosstermBackend::new(out);
        match Terminal::new(backend) {
            Ok(term) => Ok(Self { term }),
            Err(err) => {
                restore();
                Err(err)
            }
        }
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.term
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

pub fn restore() {
    if RESTORED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let mut out = stdout();
        let _ = execute!(out, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn install_panic_hook() {
    if HOOK_INSTALLED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let old = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore();
        old(info);
    }));
}
