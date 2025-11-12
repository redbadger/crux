# Cat Facts Example

This example has been updated recently in order to be ready for the upcoming release of `crux_core` v0.17. This means that it depends on the local `crux_core` and you will need to clone the repository to build it.

In particular, the example uses the `facet_typegen` feature of `crux_core` to generate shared types for each foreign language (meaning we no longer need a separate crate for the shared types). The codegen is done by a binary in the shared lib at [`./shared/src/bin/codegen.rs`](./shared/src/bin/codegen.rs).

To get going quickly you can run `just dev` in each of the shell directories (`./Android`, `./cli`, `./iOS`, `./web-nextjs`, `./web-yew`). 

The `Justfile` in each of these directories describes what is required to build the example for the respective shell.

```bash
cd ./iOS # or `./Android`, `./cli`, `./web-nextjs`, `./web-yew`
just dev
```

## Dependencies

1. `rustup target list --installed` — installs the targets listed in [`rust-toolchain.toml`](./rust-toolchain.toml) if they are not already present.

1. `cargo install just` - for running tasks.

1. `brew install xcodegen` - for generating Xcode projects.

1. `cargo install cargo-swift` - for compiling the shared library as an iOS framework in a Swift package.

1. `cargo install wasm-pack` - for compiling the shared library as a WebAssembly module.

1. `brew install pnpm` - for managing dependencies in the web shells.
