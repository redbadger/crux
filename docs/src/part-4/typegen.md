# Type generation

## Why type generation?

Declaring every type across an FFI boundary is painful. Complex types
like nested enums, generics, and rich view models are difficult or
impossible to represent directly in tools like UniFFI or
`wasm-bindgen`. And even when you _can_ declare them, maintaining the
declarations by hand as your app evolves is tedious and error-prone.

Crux sidesteps this problem by keeping the FFI surface as small as
possible. The entire core-shell interface is just three methods —
`update`, `resolve`, and `view` — and all data crosses the boundary as
serialized byte arrays (using [Bincode](https://docs.rs/bincode)). The
shell doesn't need to know the Rust types at the FFI level at all.

But the shell _does_ need to serialize events and deserialize effects
and view models on its side of the boundary. For that, it needs
equivalent type definitions in Swift, Kotlin, or TypeScript — along
with the matching serialization code. This is what type generation
provides: it inspects your Rust types and generates the corresponding
foreign types and their Bincode serialization implementations
automatically.

## How it works

Type generation uses the [Facet](https://docs.rs/facet) crate for
zero-cost reflection. Types that derive the `Facet` trait can be
introspected at build time to discover their shape — fields, variants,
generic parameters. The
[facet-generate](https://github.com/redbadger/facet-generate) crate
uses that reflection data to generate equivalent types (and their
serialization code) in Swift, Kotlin, and TypeScript.

The process has three parts:

1. **Annotate your types** — derive `Facet` on types that cross the
   FFI boundary, and use `#[effect(facet_typegen)]` on your `Effect`
   enum.
2. **Add a codegen binary to your shared crate** — a short `main`
   that registers your app and generates the foreign code.
3. **Run it** — typically via a `just typegen` recipe as part of your
   build workflow.

## Annotating your types

### Events, ViewModel, and other data types

Types that the shell needs to know about should derive `Facet` (along
with `Serialize` and `Deserialize` for the FFI serialization). Here's
the counter example:

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:10:16}}
```

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:29:32}}
```

Note the `#[repr(C)]` on the enum — this is required by Facet for
enums that cross the FFI boundary.

### The Effect type

The `Effect` enum uses the `#[effect(facet_typegen)]` attribute, which
tells the `#[effect]` macro to generate the type registration code
that the codegen binary needs:

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:18:22}}
```

The macro discovers the operation types carried by each variant (e.g.
`RenderOperation`) and registers them for type generation
automatically.

### Skipping and opaque types

Not all event variants need to cross the FFI boundary. Internal
events (ones the shell never sends) can be excluded from the generated
output with `#[facet(skip)]`:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/app.rs:38:60}}
```

In this example, `Set`, `Update`, and `UpdateBy` are internal events
— the shell never creates them, so they're skipped.

However, `Facet` must still be derivable on the _entire_ type,
including skipped variants. If a skipped variant contains a field
whose type doesn't implement `Facet` (like `crux_http::Result<...>`),
you need to mark that field with `#[facet(opaque)]` so the derive
succeeds. That's why `Set` has both `#[facet(skip)]` on the variant
and `#[facet(opaque)]` on its field.

## The codegen binary

Each shared crate includes a small binary that drives the type
generation. Here's the one from the counter example:

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/bin/codegen.rs}}
```

The key steps are:

1. **`TypeRegistry::new().register_app::<Counter>()?`** — discovers
   all types reachable from your `App` implementation (events, effects,
   view model, and the operation types they reference).
2. **`.build()?`** — produces a `CodeGenerator` with the full type
   graph.
3. **`Config::builder(name, &output_dir)`** — configures the output.
   The `name` parameter is the package/module name (e.g. `"App"` for
   Swift, `"com.crux.examples.counter"` for Kotlin, `"app"` for
   TypeScript).
4. **`.add_extensions()`** — includes helper code like `Requests.swift`
   that makes it easier to work with the generated types.
5. **`.add_runtimes()`** — includes the serialization runtime (Serde
   and Bincode implementations in the target language).
6. **`.swift(&config)?`** / **`.kotlin(&config)?`** /
   **`.typescript(&config)?`** — generates the code.

The binary also handles UniFFI binding generation for Kotlin (the
`bindgen` call), which produces the Kotlin bindings for the Rust FFI
layer.

### Cargo.toml setup

The codegen binary needs a few additions to your `shared/Cargo.toml`.

Declare the binary, gated on a `codegen` feature:

```toml
{{#include ../../../examples/counter/shared/Cargo.toml:typegen_bin}}
```

Enable `facet_typegen` in `crux_core`:

```toml
{{#include ../../../examples/counter/shared/Cargo.toml:typegen}}
```

And add `facet` as a dependency — all types that cross the FFI
boundary derive `Facet`:

```toml
{{#include ../../../examples/counter/shared/Cargo.toml:typegen_deps}}
```

## Running type generation

Type generation is typically run via [Just](https://just.systems/)
recipes. Each shell runs the codegen binary and writes the output into
a `generated/` directory inside itself. In the counter example, the
layout looks like this:

```text
examples/counter/
├── shared/            # the Crux core
├── apple/
│   └── generated/     # Swift package "App"
├── Android/
│   └── generated/     # Kotlin package "com.crux.examples.counter"
├── web-react-router/
│   └── generated/
│       └── types/     # TypeScript package "app"
└── ...
```

The package names are set in `codegen.rs` via the `Config::builder`
call — see the codegen binary above.

Each shell's `Justfile` has a `typegen` recipe. For example, the Apple
shell runs:

```sh
RUST_LOG=info cargo run \
    --package shared \
    --bin codegen \
    --features codegen,facet_typegen \
    -- \
        --language swift \
        --output-dir generated
```

The `--output-dir` is relative to the shell directory where the recipe
runs — so the generated code lands right where the shell project can
reference it. The TypeScript shells use `generated/types` to keep the
types separate from the wasm package (which lives in `generated/pkg`).

The `generated/` directories are gitignored and regenerated as part of
the build process. Each shell's `build` recipe depends on `typegen`.

## What gets generated

For each target language, the codegen produces:

- **Type definitions** — enums, structs, and their serialization code,
  matching the shape of your Rust types. For example, `Event`,
  `Effect`, `ViewModel`, and any operation types.
- **Serialization runtime** — Serde and Bincode implementations in the
  target language, so the shell can serialize events and deserialize
  effects and view models.
- **Helper extensions** — like `Requests.swift`, which provides
  convenience methods for working with effect requests.

For Swift, the output is a Swift Package. For Kotlin, it's a set of
source files alongside UniFFI bindings. For TypeScript, it's an npm
package.
