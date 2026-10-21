//! Throwaway spike (plan task 7 / bead bk-ab75): prove gleam-core's parser
//! runs behind the `gleam-frontend` feature and show the shape of its AST
//! entry points. NOT production code; deleted or promoted by ADR-001.
//!
//! Run: cargo run --features gleam-frontend --example spike_frontend
#![cfg(feature = "gleam-frontend")]

use camino::Utf8PathBuf;
use gleam_core::parse::parse_module;
use gleam_core::warning::WarningEmitter;

fn main() {
    let src = "pub fn main() { 42 }\n";
    let parsed =
        parse_module(Utf8PathBuf::from("spike.gleam"), src, &WarningEmitter::null())
            .expect("parse should succeed");

    println!("SPIKE-OK module name: {}", parsed.module.name);
    println!("SPIKE definitions: {}", parsed.module.definitions.len());
    for def in &parsed.module.definitions {
        println!("SPIKE def: {def:?}");
    }
}
