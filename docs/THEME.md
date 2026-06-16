# THEME

Semantic terminal colors for termeprompter. Source of truth: `src/theme.rs`.
Tokens map to terminal colors per theme; layouts reference tokens, never raw hex.

## Base palette (Coolors rose/plum)

`#FAE3E3  #F7D4BC  #CFA5B4  #C98BB9  #846B8A`

Palette is soft/light. The reading background is derived **dark** — `#846B8A`
mixed toward black — so long-form text holds contrast. Do not introduce brand
colors outside this palette.

## Themes

Cycle order: `rose-plum → plain → mono → high-contrast → rose-plum`
(`next`/`previous` in `theme.rs`). Names accept `-`, `_`, or no separator.

- **rose-plum** (default) — dark plum background, rose foreground, plum headings.
- **plain** — terminal default bg/fg; white accents, gray dim. Inherits your shell theme.
- **mono** — black bg, white text, gray for dim/progress. No hue.
- **high-contrast** — black bg, white text, yellow accent/progress for low vision.

## Tokens

| Token          | Role                                  | rose-plum |
|----------------|---------------------------------------|-----------|
| `Bg`           | reading background                    | `#1B1620` |
| `BgDim`        | dim panels / status background        | `#241E29` |
| `Fg`           | body text / status foreground         | `#FAE3E3` |
| `Heading`      | headings (bold) / progress fill       | `#C98BB9` |
| `Cue`          | cue marks                             | `#F7D4BC` |
| `Dim`          | de-emphasized text / progress track   | `#846B8A` |
| `Accent`       | accents                               | `#CFA5B4` |
| `StatusBg`     | status bar background                 | `#241E29` |
| `StatusFg`     | status bar foreground                 | `#FAE3E3` |
| `ProgressFill` | filled progress                       | `#C98BB9` |
| `ProgressTrack`| empty progress track                  | `#846B8A` |

Non-rose-plum themes resolve the same tokens to terminal-safe colors
(`Reset`/`White`/`Gray`/`Yellow`) — see `Theme::color` in `src/theme.rs`.
