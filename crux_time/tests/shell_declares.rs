//! The `foreign_types` of each extension in `crux_time::shell` against its
//! shipped sources.
//!
//! Type generation checks every name in `foreign_types` for a clash with the
//! generated module, so a declaration missing from the list would go
//! unchecked. This test is what keeps the lists honest: it reads the
//! module-scope declarations in each extension's source directory and asserts
//! that they are exactly the names its `foreign_types` lists.
#![cfg(feature = "facet_typegen")]

use std::collections::BTreeSet;

use crux_core::type_generation::facet::{Dir, ShellExtension};

/// Every extension the crate ships, with the name of its language.
fn extensions() -> [(&'static str, &'static ShellExtension); 4] {
    [
        ("swift", &crux_time::shell::swift::TASK),
        ("kotlin", &crux_time::shell::kotlin::COROUTINE),
        ("typescript", &crux_time::shell::typescript::TIMEOUT),
        ("csharp", &crux_time::shell::csharp::TASK),
    ]
}

/// The text of every file in the extension's source directory, for every
/// platform.
fn sources(extension: &'static ShellExtension) -> Vec<&'static str> {
    extension
        .sources
        .dirs()
        .flat_map(Dir::files)
        .map(|file| {
            file.contents_utf8()
                .expect("a shipped source file should be UTF-8")
        })
        .collect()
}

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
fn declares_lists_every_module_scope_name_the_sources_declare() {
    for (language, extension) in extensions() {
        let declared: BTreeSet<&str> = extension.foreign_types.iter().copied().collect();
        let found: BTreeSet<String> = sources(extension)
            .into_iter()
            .flat_map(|source| module_scope_names(language, source))
            .collect();
        let found: BTreeSet<&str> = found.iter().map(String::as_str).collect();

        assert_eq!(
            found, declared,
            "the {language} extension's `foreign_types` should list exactly the names its \
             sources declare at module scope"
        );
    }
}
