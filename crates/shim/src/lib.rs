//! tool_shim: forwards calls between the page and `tool_core::api::Handle`.
//!
//! Builder requirement: the shim decides nothing. Each method is one call
//! into the core. `crates/core/tests/boundary.rs` fails if a branch, a loop or
//! the `?` operator appears here; if a feature seems to need one, the rule
//! belongs in the core.
//!
//! The template forwards free functions and the page holds the state as JSON.
//! This tool holds a running encoder, so the shim holds the core's handle
//! (SECOND-ORDER.md R1.4).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Tool(tool_core::api::Handle);

#[wasm_bindgen]
impl Tool {
    #[wasm_bindgen(constructor)]
    pub fn new(strings_json: &str, glossary_md: &str, abilities_json: &str) -> Tool {
        Tool(tool_core::api::Handle::new(
            strings_json,
            glossary_md,
            abilities_json,
        ))
    }

    pub fn ui(&mut self) -> String {
        self.0.ui()
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.0.act(name, value)
    }

    pub fn feed(&mut self, left: &[u8], right: &[u8]) -> String {
        self.0.feed(left, right)
    }

    pub fn live(&mut self) -> String {
        self.0.live()
    }

    pub fn file(&self) -> Vec<u8> {
        self.0.file()
    }
}
