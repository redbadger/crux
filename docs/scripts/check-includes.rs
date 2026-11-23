#!/usr/bin/env -S cargo +nightly -Zscript
---
[package]
edition = "2024"
---
//! Fail if any `{{#include path:anchor}}` in the book names a file or anchor
//! that does not exist.
//!
//! mdBook only warns about a missing anchor, and then includes nothing. A
//! renamed or deleted `ANCHOR` in an example therefore blanks a code listing in
//! the book without an error.
//!
//! Paths resolve relative to the including page. A line-range include (`:N` or
//! `:N:M`) has no anchor to check, so the file only needs to exist.
//!
//! The script needs a nightly Rust toolchain, because cargo runs single-file
//! scripts only on nightly (`-Zscript`):
//!
//!     just check-includes        (from docs/)
//!     ./scripts/check-includes.rs

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const ERROR: &str = "\x1b[1;31m";
const NORMAL: &str = "\x1b[0m";

/// The book has more than this many includes. A smaller count means that the
/// book moved and the script looks in the wrong place.
const MIN_INCLUDES: usize = 100;

fn main() -> ExitCode {
    // The script is in docs/scripts, and the pages are in docs/src.
    let docs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the script is in docs/scripts");
    std::env::set_current_dir(docs).expect("cannot enter the docs directory");

    let mut pages = Vec::new();
    markdown_files(Path::new("src"), &mut pages);

    let mut checked = 0;
    let mut failed = false;
    for page in &pages {
        let text = String::from_utf8_lossy(&fs::read(page).expect("cannot read page")).into_owned();
        for line in text.lines() {
            for spec in includes(line) {
                checked += 1;
                if let Some(message) = check(page, spec) {
                    println!("  {ERROR}✗ {message}{NORMAL}");
                    failed = true;
                }
            }
        }
    }

    if checked < MIN_INCLUDES {
        println!(
            "  {ERROR}✗ only found {checked} includes, expected at least {MIN_INCLUDES}. Has the book moved?{NORMAL}"
        );
        failed = true;
    }
    if !failed {
        println!("  ✓ {checked} includes resolve");
    }
    ExitCode::from(u8::from(failed))
}

/// Collect the `.md` files under `dir`, in name order.
fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("cannot read directory")
        .map(|e| e.expect("cannot read directory entry"))
        .collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type().expect("cannot read file type");
        if kind.is_dir() {
            markdown_files(&path, out);
        } else if kind.is_file() && path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// The text of each `{{#include ...}}` and `{{#rustdoc_include ...}}` on one
/// line, without the braces, the directive and the spaces around it.
fn includes(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("{{#") {
        rest = &rest[start + 3..];
        let after = rest.strip_prefix("rustdoc_").unwrap_or(rest);
        let Some(after) = after.strip_prefix("include ") else {
            continue;
        };
        let Some(end) = after.find('}') else {
            continue;
        };
        if after[end..].starts_with("}}") {
            found.push(after[..end].trim_matches(' '));
            rest = &after[end + 2..];
        }
    }
    found
}

/// Return an error message if the include `spec` on `page` does not resolve.
fn check(page: &Path, spec: &str) -> Option<String> {
    let (path, anchor) = match spec.split_once(':') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (spec, None),
    };
    let page_name = page.display();
    let target = page.parent().unwrap_or(Path::new("")).join(path);
    if !target.is_file() {
        return Some(format!("{page_name} includes {path}, which does not exist"));
    }
    // A line range (`3`, `3:5`) is not an identifier, so it needs no anchor.
    let anchor = anchor.filter(|a| is_anchor_name(a))?;
    let text = String::from_utf8_lossy(&fs::read(&target).expect("cannot read included file"))
        .into_owned();
    let marker = format!("ANCHOR: {anchor}");
    let found = text.lines().any(|line| {
        line.match_indices(&marker).any(|(i, _)| {
            line[i + marker.len()..].is_empty() || line[i + marker.len()..].starts_with(' ')
        })
    });
    (!found).then(|| format!("{page_name} includes {path}:{anchor}, but {path} has no such anchor"))
}

fn is_anchor_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
