# termeprompter Agent Guide

## Project Context

Terminal-rich TUI teleprompter for scripts, speeches, video/stream notes,
tutorials, narration, and presentation text. It is local, offline,
keyboard-first, and terminal-first. The text is the primary experience; chrome
only supports reading.

## Working Rules

- Make the smallest safe change that satisfies the request.
- Keep the app local and offline: no telemetry, accounts, network features,
  Electron, or web/GUI wrapper.
- Do not add dependencies unless explicitly requested or clearly justified by
  existing project direction.
- Avoid unrelated rewrites, formatting churn, and broad refactors.
- Preserve readable terminal behavior over visual gimmicks.
- Always restore terminal state on exit or panic.
- Never panic on resize.
- Use ASCII plus normal Unicode box drawing only; do not require Nerd Font or
  private-use glyphs. Maintain an ASCII fallback where relevant.

## Visual System

Use the rose/plum palette from `docs/THEME.md`:

`#FAE3E3 #F7D4BC #CFA5B4 #C98BB9 #846B8A`

The palette is soft/light; derive dark reading backgrounds from `#846B8A`
mixed with black. Do not introduce unrelated brand colors.

## Architecture

- `src/main.rs` - entry point, module wiring, top-level error handling
- `src/cli.rs` - flags and args
- `src/config.rs` - optional `~/.config/termeprompter/config.toml`
- `src/document.rs` - loaded script model: logical lines, headings, word
  counts, cue points
- `src/parser.rs` - plain text and simple markdown parsing
- `src/importer.rs` - file browser and local/PPTX text loading into documents
- `src/presentation.rs` - rich slide rendering via Kitty graphics with
  ASCII/text fallback off Kitty
- `src/theme.rs` - semantic terminal colors and themes:
  `rose_plum`, `plain`, `mono`, `high_contrast`
- `src/terminal.rs` - raw mode guard and terminal restoration
- `src/input.rs` - keyboard events to app actions
- `src/scroll.rs` - scroll position, WPM, pause state, bounds, manual movement
- `src/timing.rs` - elapsed time, remaining estimate, WPM math
- `src/render.rs` - responsive layouts for prompt, rehearsal, and minimal modes
- `src/mirror.rs` - render-layer horizontal text reversal; never mutate source
- `src/app.rs` - app state and main loop glue

## Behavior Notes

- WPM model defaults to 135, with min 40 and max 260.
- Scrolling is fractional and deterministic.
- Pause freezes scroll state exactly.
- Mirror mode applies at the render layer only.

## Commands

Inspect `Cargo.toml` before choosing checks. Existing useful commands:

```sh
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo run -- --demo
```

Tests live in `tests/core.rs`. `./install.sh` builds release and installs to
`~/.local/bin`.

## Verification

Every patch must report checks run and results. If a check cannot run, explain
why. Use `git status --short` and `git diff` before the final report.

## Rollback

Keep changes small and reversible. Do not perform destructive repo-wide rewrites
without explicit instruction.
