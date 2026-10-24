//! Tauri command wrappers. Every body lives in `chickenscratch-services`;
//! this module only adapts Tauri's `State` extractors and event emitter.

pub mod ai;
pub mod document;
pub mod git;
pub mod io;
pub mod project;
pub mod search;
pub mod settings;
pub mod templates;
pub mod threads;

pub use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
