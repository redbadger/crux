//! Compile-time checks on the request kind an operation declares.
//!
//! The `fail/` cases in this suite are post-monomorphisation constant
//! evaluation failures, which only surface once the offending instantiation is
//! codegen'd. `trybuild` builds each case with `cargo build`, so they do.
//!
//! Their `.stderr` files include rustc's rendering of the `assert!` inside
//! `core`, which differs depending on whether the `rust-src` component is
//! installed (with it, rustc shows the source line; without, a bare note).
//! `rust-toolchain.toml` and CI both install `rust-src` so the two agree. On a
//! toolchain bump, refresh the snapshots with `TRYBUILD=overwrite`.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
