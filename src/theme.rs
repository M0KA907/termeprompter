//! Semantic terminal colors.

use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeKind {
    #[default]
    RosePlum,
    Plain,
    Mono,
    HighContrast,
}

impl FromStr for ThemeKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "rose-plum" | "rose_plum" | "roseplum" => Ok(Self::RosePlum),
            "plain" => Ok(Self::Plain),
            "mono" | "monochrome" => Ok(Self::Mono),
            "high-contrast" | "high_contrast" | "highcontrast" => Ok(Self::HighContrast),
            other => Err(format!("unknown theme `{other}`")),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Token {
    Bg,
    BgDim,
    Fg,
    Heading,
    Cue,
    Dim,
    Accent,
    StatusBg,
    StatusFg,
    ProgressFill,
    ProgressTrack,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub kind: ThemeKind,
}

impl Theme {
    pub fn new(kind: ThemeKind) -> Self {
        Self { kind }
    }

    pub fn color(&self, token: Token) -> Color {
        match self.kind {
            ThemeKind::RosePlum => match token {
                Token::Bg => Color::Rgb(0x1b, 0x16, 0x20),
                Token::BgDim | Token::StatusBg => Color::Rgb(0x24, 0x1e, 0x29),
                Token::Fg | Token::StatusFg => Color::Rgb(0xfa, 0xe3, 0xe3),
                Token::Heading | Token::ProgressFill => Color::Rgb(0xc9, 0x8b, 0xb9),
                Token::Cue => Color::Rgb(0xf7, 0xd4, 0xbc),
                Token::Dim | Token::ProgressTrack => Color::Rgb(0x84, 0x6b, 0x8a),
                Token::Accent => Color::Rgb(0xcf, 0xa5, 0xb4),
            },
            ThemeKind::Plain => match token {
                Token::Bg | Token::StatusBg => Color::Reset,
                Token::BgDim => Color::Black,
                Token::Fg | Token::StatusFg => Color::Reset,
                Token::Heading | Token::Cue | Token::Accent | Token::ProgressFill => Color::White,
                Token::Dim | Token::ProgressTrack => Color::DarkGray,
            },
            ThemeKind::Mono => match token {
                Token::Bg | Token::StatusBg => Color::Black,
                Token::BgDim => Color::Black,
                Token::Fg | Token::StatusFg | Token::Heading | Token::Cue | Token::Accent => {
                    Color::White
                }
                Token::Dim | Token::ProgressFill | Token::ProgressTrack => Color::Gray,
            },
            ThemeKind::HighContrast => match token {
                Token::Bg | Token::StatusBg => Color::Black,
                Token::BgDim => Color::Black,
                Token::Fg | Token::StatusFg | Token::Heading | Token::Cue => Color::White,
                Token::Dim | Token::ProgressTrack => Color::Gray,
                Token::Accent | Token::ProgressFill => Color::Yellow,
            },
        }
    }

    pub fn style(&self, token: Token) -> Style {
        match token {
            Token::Heading => Style::default()
                .fg(self.color(token))
                .add_modifier(Modifier::BOLD),
            Token::Bg | Token::BgDim | Token::StatusBg => Style::default().bg(self.color(token)),
            _ => Style::default().fg(self.color(token)),
        }
    }

    pub fn cycle(self) -> Self {
        self.next()
    }

    pub fn next(self) -> Self {
        let kind = match self.kind {
            ThemeKind::RosePlum => ThemeKind::Plain,
            ThemeKind::Plain => ThemeKind::Mono,
            ThemeKind::Mono => ThemeKind::HighContrast,
            ThemeKind::HighContrast => ThemeKind::RosePlum,
        };
        Self { kind }
    }

    pub fn previous(self) -> Self {
        let kind = match self.kind {
            ThemeKind::RosePlum => ThemeKind::HighContrast,
            ThemeKind::Plain => ThemeKind::RosePlum,
            ThemeKind::Mono => ThemeKind::Plain,
            ThemeKind::HighContrast => ThemeKind::Mono,
        };
        Self { kind }
    }
}
