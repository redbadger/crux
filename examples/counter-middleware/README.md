# Crux Counter Example

> [!NOTE]
> This is similar to the [Counter](../counter/README.md) example, except
> that we use it as a testbed for the latest DX experiments.
> Your mileage may vary.

This Crux Counter example is a simple multi-platform application that calls a
cloud-hosted API.

It makes HTTP requests to a shared global counter hosted at
[https://crux-counter.fly.dev](https://crux-counter.fly.dev), incrementing or
decrementing the counter value.

The [server](../counter/server/) also has an endpoint for Server Sent Events
([https://crux-counter.fly.dev/sse](https://crux-counter.fly.dev/sse)), which
signals when changes are made to the global counter value — so that when you
update the counter in one client, all the other clients will update too.

We have included an example of a
[Server Sent Events capability](./shared/src/capabilities/sse.rs), which uses
the core's ability to stream responses from the shell.

![screenshots](./counter.webp)

### Notes:

1. Please make sure you have the following rust targets installed (there is a
   [`rust-toolchain.toml`](../../rust-toolchain.toml) in the root directory of
   this repo, so you should be able to type `rustup target list --installed`, in
   or below the root directory, and these targets will be installed if they are
   not already present).

   ```txt
   aarch64-apple-darwin
   aarch64-apple-ios
   aarch64-apple-ios-sim
   aarch64-linux-android
   wasm32-unknown-unknown
   x86_64-apple-ios
   ```

2. This example currently depends on the `pnpm` package manager when generating
   types for TypeScript. We are currently revisiting the type generation for
   foreign types and so this requirement will probably go, but for now, please
   [install `pnpm`](https://pnpm.io/installation).

## Rust shared library

Build the shared core and generate the shared types for your client applications

```sh
./build.sh
```

## Web app — Leptos

If you don't have it already, install the `trunk` CLI tool:

```sh
brew install trunk
```

To build and run the [Leptos](https://leptos.dev/) web app:

```
cd web-leptos
trunk serve
```

## Web app — Remix (React)

To build and run the [Remix](https://remix.run/) web app:

```
cd web-remix
pnpm wasm:build
pnpm install
pnpm dev
```

### Notes:

On Windows if you get "ℹ️ Installing wasm-pack" it does not work. You can solve
it by installing it manually from:
https://rustwasm.github.io/wasm-pack/installer/

## Mobile app — iOS

You will need [XCode](https://developer.apple.com/xcode/), which you can get in
the Mac AppStore

```
cd iOS
xed .
```

You should be able to press "Play" to start the app in the simulator, or on an
iPhone.

### Notes:

- You may encounter this error:

  ```
  xcrun: error: invalid active developer path (/Library/Developer/CommandLineTools), missing xcrun at: /Library/Developer/CommandLineTools/usr/bin/xcrun
  ```

  If this happens, then you need to install the
  [Command Line Tools For Xcode](https://developer.apple.com/download/all/).

## Mobile app — Android

Open the `Android` folder in
[Android Studio](https://developer.android.com/studio/). If the build is
successful, you should be able to press "Play" to start the app in the
simulator.

### Notes:

- The Android Studio build might fail for a couple of known reasons:
  - A `linker-wrapper.sh` script failure<br>Ensure you have Python installed and
    your `PATH`
  - `NDK is not installed`<br>Install this via Android Studio --> Settings -->
    Appearance and Behaviour --> System Settings --> Select the "SDK Tools" tab,
    select "NDK (side by side)" and press Apply to install
- If Android studio fails to install `git`, you can set the path to your git
  binary (e.g. the homebrew one) in the preferences under Version Control > Git
