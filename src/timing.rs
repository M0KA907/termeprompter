//! Clock and ETA helpers.

use crate::scroll::{Wpm, WrapLayout};
use std::time::{Duration, Instant};

pub trait Clock {
    fn now(&self) -> Instant;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MockClock {
    pub t: Instant,
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.t
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Estimate {
    pub elapsed: Duration,
    pub remaining: Duration,
    pub progress: f64,
}

pub fn estimate(word_pos: f64, layout: &WrapLayout, wpm: Wpm, elapsed: Duration) -> Estimate {
    let total = layout.total_words.max(0.0);
    let pos = if word_pos.is_finite() {
        word_pos.clamp(0.0, total)
    } else {
        0.0
    };
    let progress = if total > 0.0 {
        (pos / total).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let secs = ((total - pos).max(0.0)) / wpm.words_per_sec().max(f64::EPSILON);
    let remaining = Duration::from_secs_f64(secs.max(0.0));
    Estimate {
        elapsed,
        remaining,
        progress,
    }
}

pub fn fmt_clock(duration: Duration) -> String {
    let secs = duration.as_secs_f64().round().max(0.0) as u64;
    format!("{}:{:02}", secs / 60, secs % 60)
}
