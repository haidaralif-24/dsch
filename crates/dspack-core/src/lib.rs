//! dspack-core: all logic, no printing, no prompting.
//!
//! The CLI and TUI front ends in the `dspack` crate are thin layers over this
//! library. See ARCHITECTURE.md for the data flow.

pub mod apply;
pub mod config;
pub mod git;
pub mod history;
pub mod pack;
pub mod parse;
pub mod plan;
pub mod root;
pub mod safety;
