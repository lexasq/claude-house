//! Rules for Claude House as pure functions: profile ids and folders, launch commands, claude:// routing,
//! sign-in state. The one exception is `store`, which reads and writes `profiles.json`.
//! No processes, no registry, no UI. Terms are defined in `docs/glossary.md`; facts about Claude Desktop
//! these rules rely on are in `docs/phase0-findings.md`.

pub mod account;
pub mod handler;
pub mod install;
pub mod launch;
pub mod link;
pub mod profile;
pub mod store;
