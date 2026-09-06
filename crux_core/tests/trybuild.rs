//! Compile-time checks on the request kind an operation declares.
//!
//! The `fail/` cases in this suite are post-monomorphisation constant
//! evaluation failures, which only surface once the offending instantiation is
//! codegen'd. `trybuild` builds each case with `cargo build`, so they do.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
