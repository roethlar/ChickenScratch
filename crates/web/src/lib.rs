//! `chikn-web`: the self-hosted web shell (`docs/plans/PLAN_WEB_SHELL.md`,
//! ADR-007).
//!
//! One process runs the engine in-process through `chickenscratch-services`,
//! serves the built React UI from `ui/dist`, and answers the UI's command
//! calls as same-origin JSON requests (`POST /api/cmd/<name>`) with exactly
//! the argument and error contract the Tauri shell has. Everything the server
//! may touch on disk is confined to one projects root (`gate`), and every API
//! request carries a bearer token (`auth`).

pub mod app;
pub mod auth;
pub mod commands;
pub mod config;
pub mod events;
pub mod export;
pub mod fs_browse;
pub mod gate;
pub mod lease;
pub mod projects;
pub mod state;
pub mod static_ui;
pub mod upload;

pub use app::build_router;
pub use gate::{Gate, GateError, PathArg};
pub use state::AppState;
