# Android

[Table of Contents](./new-project.md)

1. Create a Kotlin App in Android Studio (e.g. "Empty Compose Activity (Material UI)" at `/Android`)

1. Add a Kotlin Android Library (`aar`) — you can find more details on how to do this [here](https://developer.android.com/studio/projects/android-library), but in a nutshell

   1. Go to File -> New -> New Module...
   1. Choose "Android Library"
   1. Call it something like `shared`
   1. The package name must match that shown in [`./shared/uniffi.toml`](../shared/uniffi.toml), e.g. `com.redbadger.crux_core.shared`

1. Add the `shared` library as a dependency of `app`

   1. Either
      1. Go to File -> Project Structure...
      1. Choose "Dependencies"
      1. Choose `app` and use the `+` to add a "Module dependency"
      1. Select the `shared` library
   1. Or
      1. Add this line to the `dependencies` section in [`./Android/app/build.gradle`](../Android/app/build.gradle)
         ```groovy
         implementation project(path: ':shared')
         ```

1. Mozilla has a Rust gradle plugin for android [here](https://github.com/mozilla/rust-android-gradle). Add the plugin to `./Android/build.gradle`, and sync

   ```groovy
   plugins {
       id "org.mozilla.rust-android-gradle.rust-android" version "0.9.3"
   }
   ```

1. We are also using [Java Native Access (JNA)](https://github.com/java-native-access/jna).
   Add the following to `./Android/shared/build.gradle`, and sync

   ```groovy
   plugins {
       ...
       id 'org.mozilla.rust-android-gradle.rust-android'
   }
   android {
       namespace 'com.redbadger.crux_core.shared'
       ...
       ndkVersion '25.1.8937393'
   }

   dependencies {
       implementation "net.java.dev.jna:jna:5.12.1@aar"
       ...
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

1. Run "Build -> Make project" to make sure that everything compiles (including the shared Rust library) — you should be able to see the library object file

   ```sh
   ls Android/shared/build/rustJniLibs/android/arm64-v8a
   libshared.so
   ```

1. Try calling into the rust library from the Android app, for example ...

   1. Open `Android/app/src/main/java/com/example/android/MainActivity.kt`
   1. Add `import com.redbadger.crux_core.shared.add`
   1. Add a `class` for the callback to get Platform details ...
      ```kotlin
      class GetPlatform : Platform {
         override fun get(): String {
            return Build.BRAND + " " + Build.VERSION.RELEASE
         }
      }
      ```
   1. Call the `addForPlatform` function, e.g. in a Text UI component ...

      ```kotlin
      Text(text = addForPlatform(1u, 2u, GetPlatform()))
      ```

   1. Run the app in a simulator to show that the function in the shared library is called
