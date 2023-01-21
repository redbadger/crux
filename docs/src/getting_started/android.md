# Android — Kotlin and Jetpack Compose

These are the steps to set up Android Studio to build and run a simple Android app that calls into a shared core.

> 🚨 _SHARP EDGE WARNING_: We want to make setting up Android Studio to work with Crux really easy. As time progresses we will try to simplify and automate as much as possible, but at the moment there is some manual configuration to do. This only needs doing once, so we hope it's not too much trouble. If you know of any better ways than those we describe below (e.g. how to do Xcode project configuration from the command line), please either raise an issue (or a PR) at <https://github.com/redbadger/crux>.

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
    tasks.withType(org.jetbrains.kotlin.gradle.tasks.KotlinCompile).configureEach {
        kotlinOptions {
            freeCompilerArgs += "-Xopt-in=kotlin.RequiresOptIn"
        }
    }

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

    def composeBom = platform('androidx.compose:compose-bom:2022.10.00')
    implementation composeBom
    androidTestImplementation composeBom

    implementation("androidx.compose.material3:material3")

    // Android Studio Preview support
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")

    // UI Tests
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")

    // Optional - Integration with activities
    implementation("androidx.activity:activity-compose:1.6.1")
    // Optional - Integration with ViewModels
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.5.1")
    // Optional - Integration with LiveData
    implementation("androidx.compose.runtime:runtime-livedata")

    implementation 'org.jetbrains.kotlinx:kotlinx-coroutines-android:1.6.4'
    implementation 'org.jetbrains.kotlinx:kotlinx-coroutines-core:1.6.4'

    implementation('com.diem:client-sdk-java:1.0.5') {
        exclude group: 'org.bouncycastle', module: 'bcprov-jdk15to18'
    }
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

Edit `/Android/app/src/main/java/com/example/android/MainActivity.kt` to look like this:

```kotlin
@file:OptIn(ExperimentalUnsignedTypes::class)
package com.example.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.android.ui.theme.AndroidTheme
import com.example.counter.shared.message
import com.example.counter.shared.view
import com.example.counter.shared_types.Effect
import com.example.counter.shared_types.Event
import com.example.counter.shared_types.Requests
import com.example.counter.shared_types.Request as Req
import com.example.counter.shared_types.ViewModel as MyViewModel


class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AndroidTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    View()
                }
            }
        }
    }
}

sealed class CoreMessage {
    data class Message(val event: Event) : CoreMessage()
}

class Model : ViewModel() {
    var view: MyViewModel by mutableStateOf(MyViewModel(""))
        private set

    init {
        update(CoreMessage.Message(Event.Reset()))
    }

    fun update(msg: CoreMessage) {
        val requests: List<Req> =
            when (msg) {
                is CoreMessage.Message -> {
                    Requests.bcsDeserialize(
                        message(msg.event.bcsSerialize().toUByteArray().toList()).toUByteArray()
                            .toByteArray()
                    )
                }
            }

        for (req in requests) when (req.effect) {
            is Effect.Render -> {
                this.view = MyViewModel.bcsDeserialize(view().toUByteArray().toByteArray())
            }
        }
    }
}

@Composable
fun View(model: Model = viewModel()) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .fillMaxSize()
            .padding(10.dp),
    ) {
        Text(text = model.view.count.toString(), modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Reset())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error
                )
            ) { Text(text = "Reset", color = Color.White) }
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Increment())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                )
            ) { Text(text = "Increment", color = Color.White) }
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Decrement())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.secondary
                )
            ) { Text(text = "Decrement", color = Color.White) }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    AndroidTheme {
        View()
    }
}
```

You should then be able to run the app in the simulator, and it should look like this:

<img alt="hello world app" src="./hello_world_android.webp"  width="300">
