## Run the Example Locally

### Rust

1. Make sure you have the following rust targets installed (e.g. `rustup target add <target-name>`)

   ```txt
   aarch64-apple-darwin
   aarch64-apple-ios
   aarch64-apple-ios-sim
   aarch64-linux-android
   wasm32-unknown-unknown
   x86_64-apple-ios
   ```

1. Install the `uniffi-bindgen` binary ...

   ```sh
   cargo install uniffi_bindgen
   ```

1. Make sure the core builds

   ```sh
   cd shared
   cargo build
   # => Finished dev [unoptimized + debuginfo] target(s) in 1.40s
   ```

### Yew web app

The web application should now build and run

```
cd web
trunk serve
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
