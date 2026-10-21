//! Directive-driven runner for `wasmtest/` fixtures.
//!
//! A fixture opts in by declaring two comments:
//!   `;; invoke: <export> [args...]`
//!   `;; expect: <expected stdout>`
//! Fixtures without directives are skipped. Requires `wasmtime` on PATH;
//! skips otherwise, matching the wasmtime_runtime.rs convention.

use std::path::{Path, PathBuf};
use std::process::Command;

fn wasmtime_available() -> bool {
    Command::new("wasmtime")
        .args(["--version"])
        .output()
        .is_ok()
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/gleam-wasm; the fixtures live at repo root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root resolves")
}

#[test]
fn run_wasmtest_fixtures() {
    if !wasmtime_available() {
        eprintln!("wasmtime not found — skipping wasmtest fixtures");
        return;
    }
    let dir = repo_root().join("wasmtest");
    let mut ran = 0;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|d| d.unwrap().path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("wat"))
        .collect();
    entries.sort();

    for path in entries {
        let src = std::fs::read_to_string(&path).expect("fixture is utf-8");
        let invoke = src.lines().find_map(|l| l.strip_prefix(";; invoke: "));
        let expect = src.lines().find_map(|l| l.strip_prefix(";; expect: "));
        let (Some(invoke), Some(expect)) = (invoke, expect) else {
            continue; // no directives — manual fixture, skip
        };

        let mut parts = invoke.split_whitespace();
        let export = parts.next().expect("invoke directive needs an export name");

        let mut cmd = Command::new("wasmtime");
        cmd.arg("run").arg("--invoke").arg(export);
        for a in parts {
            cmd.arg(a);
        }
        cmd.arg(&path);

        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("spawn wasmtime for {}: {e}", path.display()));
        assert!(
            out.status.success(),
            "{}: wasmtime failed: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            expect.trim(),
            "fixture {}",
            path.display()
        );
        ran += 1;
    }

    assert!(
        ran >= 1,
        "no directive-bearing fixtures ran — is table_call.wat missing?"
    );
}
