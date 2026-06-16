//! Render-layer mirror helpers.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn mirror_row(s: &str, content_cells: u16) -> String {
    let reversed = UnicodeSegmentation::graphemes(s, true)
        .rev()
        .collect::<String>();
    let width = UnicodeWidthStr::width(reversed.as_str());
    let target = content_cells as usize;
    if width >= target {
        reversed
    } else {
        format!("{}{}", " ".repeat(target - width), reversed)
    }
}
