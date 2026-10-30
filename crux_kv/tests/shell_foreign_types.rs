//! `shell::EXTENSION.foreign_types` against the shipped sources.
//!
//! Type generation checks every name in `foreign_types` for a clash with the
//! generated module, so a declaration missing from the list would go
//! unchecked. This test is what keeps the list honest: it reads each shipped
//! source's module-scope declarations and asserts that they are exactly the
//! names `foreign_types` lists.
#![cfg(feature = "facet_typegen")]

use std::collections::BTreeSet;

const SOURCES: &[(&str, &str)] = &[
    ("swift", include_str!("../shell/swift/KeyValue.swift")),
    ("kotlin", include_str!("../shell/kotlin/KeyValue.kt")),
    (
        "typescript",
        include_str!("../shell/typescript/keyValue.ts"),
    ),
    ("csharp", include_str!("../shell/csharp/KeyValue.cs")),
];

/// Words that can come before the declaring keyword.
const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "fileprivate",
    "internal",
    "open",
    "final",
    "sealed",
    "abstract",
    "data",
    "static",
    "export",
    "default",
    "declare",
    "async",
    "partial",
    "readonly",
    "inline",
    "suspend",
];

/// Words that declare a name, which is the next word.
const DECLARING: &[&str] = &[
    "class",
    "struct",
    "record",
    "protocol",
    "enum",
    "interface",
    "object",
    "actor",
    "fun",
    "func",
    "function",
    "const",
    "let",
    "val",
    "var",
    "typealias",
    "type",
];

/// Column-0 lines that declare nothing.
const NOT_DECLARATIONS: &[&str] = &["import", "using", "package", "namespace", "extension"];

/// The names declared at module scope: every line that starts at column 0
/// with a letter, past its modifiers and attributes. A file-private Swift or
/// Kotlin declaration is counted too, which is conservative: it reserves a
/// name that could not actually clash.
fn module_scope_names(language: &str, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        if !line.starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        let mut words = line
            .split_whitespace()
            .filter(|word| !word.starts_with('@') && !MODIFIERS.contains(word))
            .peekable();
        let Some(keyword) = words.next() else {
            continue;
        };
        if NOT_DECLARATIONS.contains(&keyword) {
            continue;
        }
        assert!(
            DECLARING.contains(&keyword),
            "the {language} source has a module-scope line this test cannot read: {line:?}"
        );
        // Kotlin's `enum class Name`.
        if keyword == "enum" && words.peek() == Some(&"class") {
            words.next();
        }
        let name: String = words
            .next()
            .unwrap_or_default()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        assert!(
            !name.is_empty(),
            "the {language} source has a declaration with no name: {line:?}"
        );
        names.push(name);
    }
    names
}

#[test]
fn foreign_types_lists_every_module_scope_name_the_sources_declare() {
    let declared: BTreeSet<&str> = crux_kv::shell::EXTENSION
        .foreign_types
        .iter()
        .copied()
        .collect();
    let found: BTreeSet<String> = SOURCES
        .iter()
        .flat_map(|(language, source)| module_scope_names(language, source))
        .collect();
    let found: BTreeSet<&str> = found.iter().map(String::as_str).collect();

    assert_eq!(
        found, declared,
        "`shell::EXTENSION.foreign_types` should list exactly the names the shipped sources \
         declare at module scope"
    );
}
