# The core

With the crate in place, the core needs three things: the app itself, a small
FFI surface for the shells to call, and a codegen binary that generates the
shell-side types (and the `Core` class that drives them).

## The app

Here's the whole counter, in `shared/src/app.rs`:

```rust,noplayground
// Rust: shared/src/app.rs
{{#include ../../../examples/counter/shared/src/app.rs:app}}
```

An `Event` for each button, a `Model` holding the count, a `ViewModel` for the
shell to display, and one effect, `Render`, which tells the shell there is a
new view to show. The `Facet` derives and `#[effect(facet_typegen)]` let type
generation see the types that cross into the shell.
[A very basic app](../part-1/basic_app.md) builds this up a piece at a time,
and [Testing](../part-1/testing.md) shows how to test it.

## The manifest

The library needs a few more dependencies, and a `codegen` binary behind a
feature flag:

```toml
# TOML: shared/Cargo.toml
{{#include ../../../examples/counter/shared/Cargo.toml:manifest}}
```

The example's workspace defines more than the one we set up: drop the
`authors`, `repository`, `license` and `keywords` lines, or add them to your
`[workspace.package]`, and add `boltffi = "=0.30.1"` to your
`[workspace.dependencies]`.

## The FFI

The shells reach the core through a `CoreFfi` type, which BoltFFI exports from
`shared/src/ffi.rs`. It takes and returns bytes, and you will rarely need to
change it:

```rust,noplayground
// Rust: shared/src/ffi.rs
{{#include ../../../examples/counter/shared/src/ffi.rs}}
```

`lib.rs` exposes it:

```rust,noplayground
// Rust: shared/src/lib.rs
{{#include ../../../examples/counter/shared/src/lib.rs}}
```

BoltFFI reads `shared/boltffi.toml` to find out where to put each shell's
bindings. The example's is a good starting point, and
[Part I](../part-1/shell.md#the-boltffi-config-file) explains what each table
does:

```toml
# TOML: shared/boltffi.toml
{{#include ../../../examples/counter/shared/boltffi.toml}}
```

## The codegen

The codegen binary generates the shell-side types: `Event`, `Effect`,
`ViewModel` and everything they refer to, an `EffectHandler` interface with a
method per operation, and a `Core` class that runs the loop between the shell
and the core.

```rust,noplayground
// Rust: shared/src/bin/codegen.rs
{{#include ../../../examples/counter/shared/src/bin/codegen.rs}}
```

The line that matters is `.boltffi(...)`. It repeats what `boltffi.toml` and
`ffi.rs` already decided (the Swift module, the Kotlin package, the npm
package and the C# namespace the bindings live in), so that type generation
can bridge the generated `Core` to `CoreFfi` for you. Without it, `Core` needs
a `CoreBridge` you write yourself. Name only the languages you build shells
for.

On Swift, the `platform(...)` calls give the generated package the same
deployment target as the BoltFFI package it now depends on.
[Bridging to BoltFFI](../part-4/typegen.md#bridging-to-boltffi) covers the
options, and what changes in each platform's build.

Each shell's `just typegen` runs this binary for its own language, so there's
nothing to run yet. Time to [build a shell](./shell.md).
