# Android with Kotlin and Jetpack Compose

We want to make setting up Android Studio to work with Crux really easy. As time progresses we will try to simplify and automate as much as possible, but at the moment there is some manual configuration to do.

> This only needs doing once, so we hope it's not too much trouble, but in the future, we intend to provide some tooling to help with these set up activities. If you know of any better ways than those we describe below (e.g. how to do Android Studio project configuration from the command line), please either raise an issue (or a PR) at <https://github.com/redbadger/crux>.

## Create an Android App

The first thing we need to do is create a new Android app in Android Studio.

Open Android Studio and create a new project, for "Phone and Tablet", of type "Empty Compose Activity (Material3)". In this walk-through, we'll call it "Android" (and use a minimum SDK of API 33).

If you choose to create the app in the root folder then your repo's directory structure might now look something like this (some files elided):

```txt
.
├── Android
│  ├── app
│  │  ├── build.gradle
│  │  ├── libs
│  │  └── src
│  │     └── main
│  │        ├── AndroidManifest.xml
│  │        └── java
│  │           └── com
│  │              └── example
│  │                 └── android
│  │                    └── MainActivity.kt
│  ├── build.gradle
│  ├── gradle.properties
│  ├── local.properties
│  └── settings.gradle
├── Cargo.lock
├── Cargo.toml
├── shared
│  ├── build.rs
│  ├── Cargo.toml
│  ├── src
│  │  ├── hello_world.rs
│  │  ├── lib.rs
│  │  └── shared.udl
│  └── uniffi.toml
├── shared_types
│  ├── build.rs
│  ├── Cargo.toml
│  └── src
│     └── lib.rs
└── target
```

## Add a Kotlin Android Library

This shared Android library (`aar`) is going to allow us to call into our shared Rust library via [Java Native Access](https://github.com/java-native-access/jna).

Under `File -> New -> New Module`, choose "Android Library" and call it something like `shared`. Set the "Package name" to match that shown in `/shared/uniffi.toml`, e.g. `com.example.counter.shared`.

For more information on how to do this see <https://developer.android.com/studio/projects/android-library>.

We'll need to set it as a dependency of our app. There are two ways to do this:

1.  Either
    1. Go to `File -> Project Structure`
    1. Choose `Dependencies`
    1. Choose `app` and use the `+` to add a `Module dependency`
    1. Select the `shared` library
1.  Or
    1. Add this line to the `dependencies` section in app's `build.gradle` (`app/build.gradle`), and click "sync now".
       ```groovy
       implementation project(path: ':shared')
       ```

## Generate FFI bindings
