//! Keyboard to action mapping.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Quit,
    TogglePlay,
    SpeedUp,
    SpeedDown,
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Home,
    End,
    GotoCue(usize),
    OpenImport,
    ImportConfirm,
    ImportClose,
    ImportParent,
    ImportUp,
    ImportDown,
    ToggleMirror,
    CycleLayout,
    ThemePrev,
    ThemeNext,
    ToggleHelp,
    Reload,
    Resize(u16, u16),
    Tick,
    None,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputCtx {
    pub help_open: bool,
    pub import_open: bool,
}

pub fn map_key(ev: KeyEvent, ctx: InputCtx) -> Action {
    if ev.modifiers.contains(KeyModifiers::CONTROL) && matches!(ev.code, KeyCode::Char('c')) {
        return Action::Quit;
    }

    if ctx.help_open {
        return match ev.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') => Action::ToggleHelp,
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        };
    }

    if ctx.import_open {
        return match ev.code {
            KeyCode::Esc | KeyCode::Char('i') => Action::ImportClose,
            KeyCode::Enter => Action::ImportConfirm,
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => Action::ImportParent,
            KeyCode::Up | KeyCode::Char('k') => Action::ImportUp,
            KeyCode::Down | KeyCode::Char('j') => Action::ImportDown,
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        };
    }

    match ev.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Enter => Action::TogglePlay,
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('f') => Action::SpeedUp,
        KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('s') => Action::SpeedDown,
        KeyCode::Up | KeyCode::Left | KeyCode::Char('k') => Action::LineUp,
        KeyCode::Down | KeyCode::Right | KeyCode::Char('j') => Action::LineDown,
        KeyCode::PageUp | KeyCode::Char('u') => Action::PageUp,
        KeyCode::PageDown | KeyCode::Char('d') => Action::PageDown,
        KeyCode::Home | KeyCode::Char('g') => Action::Home,
        KeyCode::End | KeyCode::Char('G') => Action::End,
        KeyCode::Char('i') => Action::OpenImport,
        KeyCode::Char('m') => Action::ToggleMirror,
        KeyCode::Char('l') => Action::CycleLayout,
        KeyCode::Char('[') => Action::ThemePrev,
        KeyCode::Char(']') | KeyCode::Char('t') => Action::ThemeNext,
        KeyCode::Char('?') | KeyCode::Char('h') => Action::ToggleHelp,
        KeyCode::Char('r') => Action::Reload,
        KeyCode::Char(c) if ('1'..='9').contains(&c) => Action::GotoCue((c as u8 - b'1') as usize),
        _ => Action::None,
    }
}
