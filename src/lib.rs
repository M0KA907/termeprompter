//! termeprompter — terminal teleprompter TUI.
//! Module layout and shared contract are locked by the design tournament.
//! Canonical scroll position is a fractional WORD cursor; display rows derived.

pub mod app;
pub mod cli;
pub mod config;
pub mod demo;
pub mod document;
pub mod importer;
pub mod input;
pub mod mirror;
pub mod parser;
pub mod presentation;
pub mod render;
pub mod scroll;
pub mod terminal;
pub mod theme;
pub mod timing;
