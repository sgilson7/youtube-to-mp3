//! tool_core: every application rule of the tool.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `api.rs` is the one door the shim calls through.

pub mod api;
pub mod format;
pub mod glossary;
pub mod id3;
pub mod link;
pub mod page;
pub mod pcm;
pub mod recorder;
