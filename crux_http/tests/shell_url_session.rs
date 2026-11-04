//! The Apple and Linux `URLSession.swift` files against each other.
//!
//! The two files differ only in their imports: on Linux, `URLSession` is in
//! `FoundationNetworking`. This test makes sure that a change to one file is
//! also made to the other.

const APPLE: &str = include_str!("../shell/swift/url_session/apple/URLSession.swift");
const LINUX: &str = include_str!("../shell/swift/url_session/linux/URLSession.swift");

/// The lines of `source` that are not `import` statements.
fn without_imports(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| !line.starts_with("import "))
        .collect()
}

#[test]
fn the_url_session_files_differ_only_in_their_imports() {
    assert_eq!(
        without_imports(APPLE),
        without_imports(LINUX),
        "apple/URLSession.swift and linux/URLSession.swift should be the same apart from their \
         imports"
    );
}

#[test]
fn only_the_linux_file_imports_foundation_networking() {
    assert!(!APPLE.contains("import FoundationNetworking"));
    assert!(LINUX.contains("\nimport FoundationNetworking\n"));
}
