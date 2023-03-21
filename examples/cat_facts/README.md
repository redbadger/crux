## Run the Cat Facts Example Locally

Note: Whilst this example _does_ work, the API that it uses is not under our control and can be flaky, so your mileage may vary. I would look at the [Counter](../counter/README.md) example first.

### Notes:

1. Please make sure you have the following rust targets installed (there is a [`rust-toolchain.toml`](../../rust-toolchain.toml) in the root directory of this repo, so you should be able to type `rustup target list --installed`, in or below the root directory, and these targets will be installed if they are not already present).

   ```txt
   aarch64-apple-darwin
   aarch64-apple-ios
   aarch64-apple-ios-sim
   aarch64-linux-android
   wasm32-unknown-unknown
   x86_64-apple-ios
   ```

2. This example currently depends on the `pnpm` package manager when generating types for TypeScript. We are currently revisiting the type generation for foreign types and so this requirement will probably go, but for now, please [install `pnpm`](https://pnpm.io/installation).

### Rust

1. Make sure the core builds

   ```sh
   cargo build --package shared
   # => Finished dev [unoptimized + debuginfo] target(s) in 1.40s
   ```

2. Generate the shared types for your client applications

   ```sh
   cargo build --package shared_types
   ```

### Yew web app

The web application should now build and run

```
cd web-yew
trunk serve
```

### React web app

The web application should now build and run

```
cd web-nextjs
pnpm install
pnpm dev
```

### iOS

You will need XCode, which you can get in the mac AppStore.
When XCode starts, open the `iOS` directory and run a build, the app should start in the simulator.

### Android

You will need [Android Studio](https://developer.android.com/studio/).
You might face a few problems:

- The build fails due to a `linker-wrapper.sh` script failure.
  Make sure you have Python installed and your `PATH`
- Android studio fails to install `git`.
  You can set the path to your git binary (e.g. the homebrew one) in the preferences under Version Control > Git

You should be able to build and run the project in the simulator.
