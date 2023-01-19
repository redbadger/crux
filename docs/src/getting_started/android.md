# Android — Kotlin and Jetpack Compose

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

This shared Android library (`aar`) is going to wrap our shared Rust library.

Under `File -> New -> New Module`, choose "Android Library" and call it something like `shared`. Set the "Package name" to match the one from your `/shared/uniffi.toml`, e.g. `com.example.counter.shared`.

For more information on how to add an Android library see <https://developer.android.com/studio/projects/android-library>.

We can now add this library as a _dependency_ of our app.

> Don't just copy and paste the groovy snippets on this page — instead, ensure that each section has (at least) the contents shown.

Merge the following into the **app**'s `build.gradle` (`/Android/app/build.gradle`).

```groovy
android {
    packagingOptions {
        resources {
            excludes += '/META-INF/{AL2.0,LGPL2.1}'
            // this prevents an error with duplicate META-INF/DEPENDENCIES
            excludes += '/META-INF/DEPENDENCIES'
        }
    }
}

dependencies {
    // our shared library
     implementation project(path: ':shared')

    def lifecycle_version = "2.5.1"

    // ViewModel
    implementation "androidx.lifecycle:lifecycle-viewmodel-ktx:$lifecycle_version"
    // ViewModel utilities for Compose
    implementation "androidx.lifecycle:lifecycle-viewmodel-compose:$lifecycle_version"

    // Optional - Integration with activities
    implementation("androidx.activity:activity-compose:1.6.1")

    implementation 'org.jetbrains.kotlinx:kotlinx-coroutines-android:1.6.4'
    implementation 'org.jetbrains.kotlinx:kotlinx-coroutines-core:1.6.4'

    implementation 'androidx.core:core-ktx:1.7.0'

    implementation "androidx.compose.ui:ui:$compose_version"
    implementation "androidx.compose.ui:ui-tooling-preview:$compose_version"
    implementation 'androidx.compose.material3:material3:1.0.0-alpha02'
    testImplementation 'junit:junit:4.13.2'
    androidTestImplementation 'androidx.test.ext:junit:1.1.5'
    androidTestImplementation 'androidx.test.espresso:espresso-core:3.5.1'
    androidTestImplementation "androidx.compose.ui:ui-test-junit4:$compose_version"
    debugImplementation "androidx.compose.ui:ui-tooling:$compose_version"
    debugImplementation "androidx.compose.ui:ui-test-manifest:$compose_version"
}
```

## The Rust shared library

We'll use the following tools to incorporate our Rust shared library into the Android library added above. This includes compiling and linking the Rust dynamic library and generating the runtime bindings and the shared types (including copying them into our project).

- The [Android NDK](https://developer.android.com/ndk)
- Mozilla's [Rust gradle plugin](https://github.com/mozilla/rust-android-gradle) for Android
- [Java Native Access](https://github.com/java-native-access/jna)
- [Uniffi](https://mozilla.github.io/uniffi-rs/) to generate Java bindings
- `com.novi.serde`, which is part of the [diem client SDK](https://javadoc.io/doc/com.diem/client-sdk-java/latest/index.html), which we'll need for serialization

Let's get started.

> Don't just copy and paste the groovy snippets on this page — instead, ensure that each section has (at least) the contents shown.

Merge the following into the **project**'s `build.gradle` (`/Android/build.gradle`).

```groovy
buildscript {
    ext {
        compose_version = '1.3.3'
    }
}

plugins {
    id "org.mozilla.rust-android-gradle.rust-android" version "0.9.3"
}
```

> Don't just copy and paste the groovy snippets on this page — instead, ensure that each section has (at least) the contents shown.

Merge the following into the **library**'s `build.gradle` (`/Android/shared/build.gradle`).

```groovy
plugins {
    id 'org.mozilla.rust-android-gradle.rust-android'
}
android {
    ndkVersion "25.1.8937393"
}

dependencies {
    implementation "net.java.dev.jna:jna:5.12.1@aar"

    // for com.novi.serde
    implementation('com.diem:client-sdk-java:1.0.5') {
        exclude group: 'org.bouncycastle', module: 'bcprov-jdk15to18'
    }
}

apply plugin: 'org.mozilla.rust-android-gradle.rust-android'

cargo {
   module  = "../.."
   libname = "shared"
   targets = ["arm64"]
   extraCargoBuildArguments = ['--package', 'shared']
}

afterEvaluate {
   // The `cargoBuild` task isn't available until after evaluation.
   android.libraryVariants.all { variant ->
      def productFlavor = ""
      variant.productFlavors.each {
            productFlavor += "${it.name.capitalize()}"
      }
      def buildType = "${variant.buildType.name.capitalize()}"
      tasks["cargoBuild"].dependsOn(tasks["bindGen"])
      tasks["typesGen"].dependsOn(tasks["cargoBuild"])
      tasks["generate${productFlavor}${buildType}Assets"].dependsOn(tasks["typesGen"], tasks["cargoBuild"])
   }
}

task bindGen(type: Exec) {
   def outDir = "${projectDir}/src/main/java"
   workingDir "../../"
   commandLine(
            "sh", "-c",
            """\
            \$HOME/.cargo/bin/uniffi-bindgen generate shared/src/shared.udl \
            --language kotlin \
            --out-dir $outDir
            """
   )
}

task typesGen(type: Exec) {
   def outDir = "${projectDir}/src/main/java"
   def srcDir = "shared_types/generated/java/com"
   workingDir "../../"
   commandLine(
            "sh", "-c",
            """\
            cp -r $srcDir $outDir
            """
   )
}

```

> When you have edited the gradle files, don't forget to click "sync now".

If you now build your project you should see the shared library object file, and the shared types, in the right places.

```sh
$ ls --tree Android/shared/build/rustJniLibs
Android/shared/build/rustJniLibs
└── android
   └── arm64-v8a
      └── libshared.so

$ ls --tree Android/shared/src/main/java/com/example/counter
Android/shared/src/main/java/com/example/counter
├── shared
│  └── shared.kt
└── shared_types
   ├── Effect.java
   ├── Event.java
   ├── RenderOperation.java
   ├── Request.java
   ├── Requests.java
   ├── TraitHelpers.java
   └── ViewModel.java
```

## Create some UI and run in the Simulator

### Hello World counter example

There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of UI for Android in the Crux repository. The simplest is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world), but this deliberately does not have an Android example.
