# termeprompter

Terminal-native teleprompter for scripts, talks, narration, and presentation notes.

## Run From Source

```sh
cargo run
cargo run -- --demo
cargo run -- --slides-demo
cargo run -- path/to/script.txt
```

Running without arguments opens only the terminal file browser. The prompter view
appears after you open a supported file or import a folder. Demo text only loads
with `--demo`. The bundled PowerPoint demo loads with `--slides-demo`, or can be
opened directly from `examples/slides-demo.pptx`.

In Kitty, PowerPoint files (`.ppt`, `.pptx`, `.pps`, `.ppsx`) open in a rich
slide mode rendered locally through LibreOffice and Kitty graphics. Use
Space/Enter/Right/Down/PageDown for next slide and Left/Up/PageUp for previous.

Horizontal is the default layout. It uses a tape-style reader with the active word held near the
center. Trainer layout switches to a speedreading drill that shows one paced
word chunk at a time.

Useful keys:

- `p`, Space, or Enter: play/pause
- `j`/`k` or Up/Down: move one row
- `u`/`d` or PageUp/PageDown: move one page
- `s`/`-` and `f`/`+`: slower/faster WPM
- `g`/`G` or Home/End: start/end
- `1`-`9`: jump to cues
- `i`: import menu
- `m`: mirror mode
- `l`: cycle horizontal, trainer, prompt, rehearsal, and minimal layouts
- `[`/`]` or `t`: previous/next theme
- `?` or `h`: help controls in the bottom bar
- `q` or Esc: quit

## Install

On Arch Linux or Ubuntu:

```sh
./install.sh
```

The installer builds `target/release/termeprompter` and installs it to
`/usr/local/bin/termeprompter`. If `cargo` is missing, it installs Rust tooling
with `pacman` on Arch or `apt-get` on Ubuntu.

Install without root by using a user-local prefix:

```sh
./install.sh --prefix "$HOME/.local"
```

Make sure `~/.local/bin` is on your `PATH`, then run:

```sh
termeprompter --demo
termeprompter path/to/script.txt
```

Skip package-manager dependency installation:

```sh
./install.sh --no-deps
```

Uninstall:

```sh
./install.sh --uninstall
```

For a user-local install:

```sh
./install.sh --prefix "$HOME/.local" --uninstall
```
