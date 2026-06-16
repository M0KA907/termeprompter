# termeprompter

<p align="center">
  <img alt="Rust 2021" src="https://img.shields.io/badge/Rust-2021-C98BB9?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="Ratatui 0.29" src="https://img.shields.io/badge/Ratatui-0.29-CFA5B4?style=for-the-badge">
  <img alt="Crossterm 0.28" src="https://img.shields.io/badge/Crossterm-0.28-F7D4BC?style=for-the-badge">
  <img alt="Terminal first" src="https://img.shields.io/badge/Platform-terminal-846B8A?style=for-the-badge&logo=gnubash&logoColor=white">
  <img alt="Offline app" src="https://img.shields.io/badge/Offline-local_only-FAE3E3?style=for-the-badge">
  <img alt="Version 0.1.0" src="https://img.shields.io/badge/Version-0.1.0-CFA5B4?style=for-the-badge">
</p>

**termeprompter** is a local terminal teleprompter for scripts, speeches,
narration, stream notes, tutorials, and slide talks. It runs in your terminal,
keeps everything offline, and gives you a paced reading view without accounts,
telemetry, or a browser wrapper.

## Install

User-local install:

```sh
git clone https://github.com/M0KA907/termeprompter.git && cd termeprompter && ./install.sh --prefix "$HOME/.local"
```

Then run:

```sh
termeprompter --demo
```

If `~/.local/bin` is not on your `PATH`, add it for your shell:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

System install:

```sh
git clone https://github.com/M0KA907/termeprompter.git && cd termeprompter && ./install.sh
```

The installer builds a release binary with Cargo. If Cargo is missing, it can
install Rust tooling with `pacman` on Arch Linux or `apt-get` on Ubuntu. To skip
that and use only an existing Rust install:

```sh
./install.sh --no-deps
```

Uninstall:

```sh
./install.sh --prefix "$HOME/.local" --uninstall
```

## What This Repo Is

This repo contains a Rust TUI app built with Ratatui and Crossterm. The app is
meant for people who want to read prepared text from a terminal during a talk,
recording, stream, lesson, or rehearsal.

termeprompter focuses on:

- Local files and offline use
- Keyboard-first control
- Smooth paced reading by words per minute
- Simple script navigation with cue points
- Multiple reading layouts
- Mirror mode for teleprompter glass or camera rigs
- Terminal-safe rendering with ASCII fallback

It is not a cloud prompter, video editor, account service, Electron app, or web
dashboard.

## Repository Description

Use this for the GitHub repo description:

```text
Local terminal teleprompter for scripts, talks, narration, streams, and slide notes.
```

## Run From Source

```sh
cargo run -- --demo
cargo run -- path/to/script.md
cargo run -- --slides-demo
cargo run
```

Running without arguments opens the terminal file browser. Demo text only loads
with `--demo`.

## Supported Inputs

- Text: `.txt`, `.text`, `.md`, `.markdown`, `.rst`, `.adoc`, `.asc`
- PowerPoint text import: `.pptx`, `.ppsx`
- PowerPoint rich slide mode in Kitty: `.ppt`, `.pptx`, `.pps`, `.ppsx`
- Folders containing supported text files

Legacy binary `.ppt` and `.pps` files need Kitty rich slide mode. For normal
text import, save them as `.pptx` or `.ppsx`.

## Controls

| Key | Action |
| --- | --- |
| `p`, Space, Enter | Play or pause |
| `j` / `k`, Up / Down | Move one row |
| `u` / `d`, PageUp / PageDown | Move one page |
| `s` / `-`, `f` / `+` | Slower or faster WPM |
| `g` / `G`, Home / End | Jump to start or end |
| `1`-`9` | Jump to cue points |
| `i` | Open import menu |
| `m` | Toggle mirror mode |
| `l` | Cycle layouts |
| `[` / `]`, `t` | Cycle themes |
| `?`, `h` | Show help |
| `q`, Esc, Ctrl-C | Quit |

Slide mode:

| Key | Action |
| --- | --- |
| Space, Enter, Right, Down, PageDown | Next slide |
| Left, Up, PageUp | Previous slide |
| Home / End | First or last slide |

## Options

```sh
termeprompter [OPTIONS] [PATH]

Options:
  --demo                 Use built-in demo text
  --slides-demo          Use built-in PowerPoint slide demo
  --wpm <WPM>            Words per minute, 40 through 1000
  --theme <THEME>        rose-plum, plain, mono, high-contrast
  --layout <LAYOUT>      horizontal, trainer, prompt, rehearsal, minimal
  --mirror               Start mirrored
  --ascii                Use ASCII-only chrome
  -h, --help             Show help
  -V, --version          Show version
```

## Cue Points

Add cue markers to a script, then press `1` through `9` to jump between them:

```md
# Launch Talk

[cue:intro]
Welcome everyone.

[cue:demo]
Now switch to the live demo.
```

## Development

```sh
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## License

MIT. See `Cargo.toml`.
