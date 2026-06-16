//! Built-in demo assets.

use anyhow::Context;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEXT: &str = r#"# Orbital Product Briefing

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

const SLIDES_DEMO: &[u8] = include_bytes!("../examples/slides-demo.pptx");

pub fn write_slides_demo() -> anyhow::Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before UNIX_EPOCH")?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "termeprompter-slides-demo-{}-{now}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join("slides-demo.pptx");
    fs::write(&path, SLIDES_DEMO).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}
