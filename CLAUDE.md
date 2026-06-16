# termeprompter

## Task Context
Terminal-rich TUI teleprompter. Reads scripts, speeches, video/stream notes,
tutorials, narration, presentation text directly inside the terminal.
Local, offline, keyboard-first. The text is the star; chrome supports reading.

## Assumptions
Rust + Ratatui unless existing repo code proves otherwise.
Repo was empty at init (only `.remember/`). Clean scaffold.

## Design Constraints
Local, offline, terminal-first, keyboard-first.
No telemetry, no accounts, no network, no Electron, no web/GUI wrapper.
No Nerd Font / private-use glyphs. ASCII + normal Unicode box drawing only,
with ASCII fallback. Never panic on resize. Always restore terminal on exit/panic.

## Visual System
Rose/plum Coolors palette:
`#FAE3E3 #F7D4BC #CFA5B4 #C98BB9 #846B8A`
Palette is soft/light — derive DARK backgrounds from `#846B8A` mixed with black
for long-reading contrast. Do not invent unrelated brand colors. See `docs/THEME.md`.

## Architecture
- `main.rs` — entry, wire modules, top-level error handling
- `cli.rs` — parse flags/args (clap)
- `config.rs` — load `~/.config/termeprompter/config.toml` (optional, not required for MVP)
- `document.rs` — loaded script: logical lines, headings, word counts, cue points
- `parser.rs` — plain text + simple markdown parsing
- `importer.rs` — file browser + local/pptx text loading into Document
- `presentation.rs` — rich slide rendering via Kitty graphics; ASCII/text fallback off-Kitty
- `theme.rs` — semantic tokens -> terminal colors; themes rose_plum/plain/mono/high_contrast
- `terminal.rs` — raw mode enter/exit guard; restore even on error
- `input.rs` — keyboard events -> app actions
- `scroll.rs` — scroll position, WPM, pause state, bounds, manual movement
- `timing.rs` — elapsed, remaining estimate, WPM math
- `render.rs` — draw layouts (prompt/rehearsal/minimal), responsive sizing
- `mirror.rs` — render-layer horizontal text reversal (never mutate source)
- `app.rs` — app state + main loop glue

## Commands
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo run -- --demo
# tests live in tests/core.rs; ./install.sh builds release + installs to ~/.local/bin

## Testing Proof
Every patch must report checks run + results. If a check cannot run, explain why.

## Rollback
Small commits. No destructive repo-wide rewrites without explicit instruction.

## Memory / Context
Preserve readable terminal behavior over visual gimmicks.
WPM speed model: default 135, min 40, max 260. Fractional scroll, deterministic.
Pause freezes scroll state exactly. Mirror applies at render layer only.
