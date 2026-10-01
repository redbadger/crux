# Upgrading from 0.20 to 0.21

This is a checklist for moving an app from `crux_core` 0.20 to 0.21. Work down
it in order: first the dependencies, then what stops your app compiling, then
what to check when you regenerate your shells. Everything after that is
optional.

The largest change, operations as types, has its own
[migration guide](./migrate-per-operation-types.md). You don't have to do it
to upgrade: the enum APIs of `crux_kv` and `crux_time` are deprecated, but they
still work.

---

## 1. Update your dependencies

Each capability crate is built against one `crux_core` minor version, so move
them all together:

| Crate | 0.20 | 0.21 |
| --- | --- | --- |
| `crux_core` | 0.20.0 | 0.21.0 |
| `crux_http` | 0.20.0 | 0.21.0 |
| `crux_kv` | 0.14.0 | 0.15.0 |
| `crux_time` | 0.18.0 | 0.19.0 |
| `crux_macros` | 0.10.1 | 0.11.0 |

You only need `crux_macros` if you depend on it directly; `crux_core`
re-exports its macros.

```toml
# Cargo.toml
[dependencies]
crux_core = "0.21"
crux_http = "0.21"
crux_kv = "0.15"
crux_time = "0.19"
```

Then check the pins around them:

- [ ] **`facet`** stays at `=0.46.5`.
- [ ] **`facet_generate`**: `crux_core`'s `facet_typegen` feature now pulls in
      0.22 (it was 0.19). Change it only if your own crate depends on
      `facet_generate`. `facet-generate-attrs` stays at 0.18.
- [ ] **BoltFFI** moves from 0.29.3 to 0.30.1. Move all three pieces at once:
      the `boltffi` crate (`boltffi = "=0.30.1"`), the CLI
      (`cargo install boltffi_cli --version '=0.30.1' --locked`) and
      `@boltffi/runtime` 0.30.1 in each web shell's `package.json`. `crux_core`
      doesn't depend on BoltFFI, so Cargo won't catch a mismatch for you, and a
      runtime that doesn't match the crates breaks the web shells.

---

## 2. Fix what no longer compiles

### The serde-based type generation is gone

Facet type generation has been the documented path since 0.19, and it is now
the only one.

- [ ] Replace the `typegen` feature with `facet_typegen`, on `crux_core` and on
      each capability crate you enable it for (`crux_http`, `crux_kv`,
      `crux_time`, `crux_macros`).
- [ ] Replace `#[effect(typegen)]` with `#[effect(facet_typegen)]`. The old form
      is now a compile error that tells you so.
- [ ] Remove any `#[derive(Export)]`, and any use of `crux_core::typegen`,
      `crux_core::type_generation::serde` or `Operation::register_types`.
      `Operation::register_types_facet` keeps its name.
- [ ] Derive `Facet` on the types that cross the bridge, and generate from a
      codegen binary using `TypeRegistry`. [Type generation](../part-4/typegen.md)
      covers [the annotations](../part-4/typegen.md#annotating-your-types),
      [skipped and opaque types](../part-4/typegen.md#skipping-and-opaque-types)
      and [the codegen binary](../part-4/typegen.md#the-codegen-binary).

### `ResolveError` has grown

- [ ] `ResolveError` is now `#[non_exhaustive]`, so an exhaustive `match` on it
      needs a `_` arm.
- [ ] Resolving a notification through the bridge now fails with
      `ResolveError::Never`. In 0.20 the bridge reported `NotFound`, so update
      any test that expects that. `NotFound` now only
      means an id that was never issued or has already been resolved.
- [ ] Three new variants, `NoSuchEffect`, `WrongEffect` and `WrongKind`, report
      an id that doesn't belong to the request it claims to answer.

### Request ids are no longer a counter

In 0.20 an id was a counter that went up by one for every request. It now
carries the effect variant, the operation kind and a sequence, and every
notification shares id 0.

- [ ] If a shell or a test makes up an id, or expects ids to be `0`, `1`, `2`…,
      change it to use the id that arrived with the request. There's nothing in
      the id for a shell to read. See [request ids](../part-4/typegen.md#request-ids).

### An effect enum has at most 256 variants

- [ ] `#[effect]` rejects an enum with more, because the variant index takes
      eight bits of the request id. It's a compile error, so you'll know.

### `Render` and HTTP only go through their own `Command` constructor

`RenderOperation` now declares itself a notification and `HttpRequest` a
request.

- [ ] If you send either with a different constructor (`request_from_shell`
      for a render, say), that's now a build error. It's reported by
      `cargo build`, `cargo test` or `cargo clippy --all-targets`, not by
      `cargo check`. Your own operations aren't affected unless you give them
      a kind.

### Deprecation warnings

- [ ] The enum APIs of `crux_kv` (`KeyValue`, `KeyValueOperation`, …) and
      `crux_time` (`Time`, `TimeRequest`, …) are deprecated. If you build with
      warnings as errors, either migrate (see the
      [migration guide](./migrate-per-operation-types.md)) or put
      `#[allow(deprecated)]` on the code that uses them for now. The
      [deprecations table](./migrate-per-operation-types.md#deprecations) names
      each replacement.

---

## 3. Regenerate your shells, and check them

Type generation now emits more than your types: an operation-kind accessor on
each effect, an `EffectHandler` and `EffectDispatcher`, and, when your effect
enum has a `Render` variant, a `Core` and a `CoreBridge`. All of it is
additive. A shell that matches on `Effect` and calls `resolve` by hand keeps
working. A few things can still trip you up:

- [ ] **Reserved names.** A shared type called `OperationKind`,
      `EffectHandler`, `IEffectHandler`, `EffectSink`, `IEffectSink`,
      `EffectDispatcher`, `Core`, `CoreBridge`, `ICoreBridge` or `FfiBridge`,
      or an effect variant called `OperationKind`, is now an error from
      `TypeRegistry::build`. Rename it with `#[facet(rename = "...")]`.
- [ ] **Two types with the same generated name** are now an error. 0.20 kept
      the first and silently dropped the other. Rename one, or give it a
      namespace of its own.
- [ ] **A `Core` of your own.** The generated package now has a class called
      `Core`. On Android, a hand-written `Core` in the same Kotlin package is a
      real clash: the two share a fully-qualified name, D8 fails release builds
      with "Type … Core is defined multiple times", and debug builds put both
      in the APK and crash on launch. In Swift and TypeScript yours shadows or
      sits beside the generated one without breaking the build, but renaming it
      is still clearer. Rename yours (the tutorial uses `CoreWrapper`), or turn
      the generated one off with `CodeGenerator::without_core()`.
- [ ] **Kotlin.** The generated module's `build.gradle.kts` now declares
      `kotlinx-coroutines-core`.
- [ ] **TypeScript.** `CodeGenerator::typescript` now fails if `pnpm install`
      or `tsc` fails in the generated package. Before, it carried on, so a
      generated package that doesn't type-check now fails at typegen instead
      of in your shell's build.

---

## 4. Adopt what's new

None of this is required, but it removes most of the code a shell writes.

- [ ] **Per-operation types.** Move to `crux_kv::store::KeyValue`,
      `crux_time::clock::Time` and one effect variant per operation, and use
      `#[derive(Operation)]` for your own capabilities. The
      [migration guide](./migrate-per-operation-types.md) walks through it,
      and [Building capabilities](../part-2/capabilities.md) explains the
      design.
- [ ] **The generated `Core`.** Implement the generated `EffectHandler` and let
      `Core` run the loop your shell used to write by hand. See
      [who drives the loop](../part-2/shell.md#who-drives-the-loop) and
      [the generated Core](../part-4/typegen.md#the-generated-core).
- [ ] **The generated BoltFFI bridge.** Add `.boltffi(BoltFfi::new()...)` to
      your codegen binary, and `Core` gets a constructor that takes only your
      handler. See [bridging to BoltFFI](../part-4/typegen.md#bridging-to-boltffi).
      Two build changes come with it:
  - TypeScript: the generated `package.json` now depends on the wasm
    package, and typegen runs `pnpm install`, so run `boltffi pack wasm`
    **before** typegen.
  - Swift: the generated package depends on BoltFFI's, so give it a
    `platforms:` floor with `Config::builder(..).platform(".iOS(.v16)")`.
- [ ] **Shell handlers from the capability crates.** `crux_http`, `crux_kv` and
      `crux_time` come with their shell side in Swift, Kotlin, TypeScript and
      C#. Register them in your codegen binary with `.shell_handler(&crux_http::HTTP)?` (and
      `crux_kv::KEY_VALUE`, `crux_time::TIME`), and each of your handler's
      methods becomes a one-line delegation. See
      [shipped shell handlers](../part-4/typegen.md#shipped-shell-handlers)
      and [adopting the shipped handlers](./migrate-per-operation-types.md#adopting-the-shipped-handlers).
      Registering `crux_kv::KEY_VALUE` puts `GetValue`, `SetValue`,
      `DeleteValue`, `KeyExists` and `ListKeys` into your generated
      namespace, so rename any of your own types with those names.
- [ ] **The operation kind in middleware.** `resolver.kind()` tells effect
      middleware whether it holds a notification, a request or a stream. See
      [Middleware](../part-3/middleware.md).
- [ ] **HTTP from your own operations.** If you have an operation of your own
      that the shell answers with `HttpResult`, `crux_http` can now turn it
      into the same `Response` or `HttpError` a `crux_http` request gives you.
      See [Handling crux_http rejections](./http-rejections.md#your-own-operations-that-answer-with-httpresult).

---

The [one trait per operation kind RFC](../rfcs/operation-kind-traits.md)
proposes further changes to how operations are declared, for a later release;
nothing in it applies to 0.21.
