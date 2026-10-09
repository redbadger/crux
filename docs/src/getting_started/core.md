# The core

With the crate in place, the core needs three things: the app itself, a small
FFI surface for the shells to call, and a codegen binary that generates the
shell-side types (and the `Core` class that drives them). The FFI and the
codegen binary go in a second crate, next to `shared`.

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

`lib.rs` exposes the app, along with Crux's `Core` for the Rust shells:

```rust,noplayground
// Rust: shared/src/lib.rs
{{#include ../../../examples/counter/shared/src/lib.rs:lib}}
```

## The manifest

The library needs a few more dependencies, and a feature that turns on type
generation:

```toml
# TOML: shared/Cargo.toml
{{#include ../../../examples/counter/shared/Cargo.toml:manifest}}
```

The example's workspace defines more than the one we set up: drop the
`authors`, `repository`, `license` and `keywords` lines, or add them to your
`[workspace.package]`, and add `facet = "=0.46.5"` and `boltffi = "=0.30.1"`
to your `[workspace.dependencies]`.

## The FFI

The shells reach the core through a `CoreFfi` type, which BoltFFI exports from
a second crate, `ffi`. Create it next to `shared`, and add `"ffi"` to the
workspace `members`:

```sh
cargo new --lib ffi --name shared_ffi
```

Its manifest depends on `shared` and BoltFFI, and declares a `codegen` binary
behind a feature flag:

```toml
# TOML: ffi/Cargo.toml
{{#include ../../../examples/counter/ffi/Cargo.toml:manifest}}
```

Note the `crate-type` in the `[lib]` section. This is in preparation for
linking with the shells:

- `staticlib` is a static library (`libshared_ffi.a`) for use with Apple apps
- `cdylib` is a C-ABI dynamic library (`libshared_ffi.so`) for use with Android
  and other native shells, and the Wasm module for the web

Keeping these in their own crate leaves `shared` an ordinary Rust library,
which the Rust shells depend on directly.

`CoreFfi` is the whole of `ffi/src/lib.rs`. It takes and returns bytes, and
you will rarely need to change it:

```rust,noplayground
// Rust: ffi/src/lib.rs
{{#include ../../../examples/counter/ffi/src/lib.rs}}
```

BoltFFI reads `ffi/boltffi.toml` to find out where to put each shell's
bindings. The example's is a good starting point, and
[Part I](../part-1/shell.md#the-boltffi-config-file) explains what each table
does:

```toml
# TOML: ffi/boltffi.toml
{{#include ../../../examples/counter/ffi/boltffi.toml}}
```

## The codegen

The codegen binary generates the shell-side types: `Event`, `Effect`,
`ViewModel` and everything they refer to, an `EffectHandler` interface with a
method per operation, and a `Core` class that runs the loop between the shell
and the core.

```rust,noplayground
// Rust: ffi/src/bin/codegen.rs
{{#include ../../../examples/counter/ffi/src/bin/codegen.rs}}
```

The line that matters is `.boltffi(...)`. It repeats what `boltffi.toml` and
`CoreFfi` already decided (the Swift module, the Kotlin package, the npm
package and the C# namespace the bindings live in), so that type generation
can bridge the generated `Core` to `CoreFfi` for you. Without it, `Core` needs
a `CoreBridge` you write yourself. Name only the languages you build shells
for.

On Swift, the `platform(...)` calls give the generated package the same
deployment target as the BoltFFI package it depends on.
[Bridging to BoltFFI](../part-4/typegen.md#bridging-to-boltffi) covers the
options, and what changes in each platform's build.

Each shell's `just typegen` runs this binary for its own language, so there's
nothing to run yet. Time to [build a shell](./shell.md).
