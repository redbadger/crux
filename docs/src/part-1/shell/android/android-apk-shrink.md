# Reduce APK size on Android

## Build the native library in release mode

The biggest reduction in the APK size comes from building the native library in
release mode. With BoltFFI, pass `--release` when you package the Android
artifacts:

```sh
boltffi pack android --release
```

This produces an optimized `libshared.so` for each ABI. The type generation step
(the `codegen` binary) doesn't need a release profile.

## Android minify

The following is more experimental and may take some trial and error to work.

[Enable minify](https://developer.android.com/studio/build/shrink-code)
in release mode in `app/build.gradle.kts`:

```diff
buildTypes {
    release {
-        isMinifyEnabled = false
+        isMinifyEnabled = true
        proguardFiles(
            getDefaultProguardFile("proguard-android-optimize.txt"),
            "proguard-rules.pro"
        )
    }
}
```

Minification can remove generated bridge code. Add a keep rule for the shared
package in `proguard-rules.pro`:

```
# BoltFFI generates JNI bindings (a `jni_glue.c` plus Kotlin classes in your
# shared package). Keep those generated classes so R8/ProGuard doesn't strip the
# code that bridges into the native library.
#
# If you have some iOS/other non-Android functions in the shared package, you may
# want to exclude them here.
-keep class <shared app package name>.** {
  public protected *;
}
```

If this still crashes at runtime, expand the rules to include more functions or
classes. These references cover the rule syntax and common Android/Rust cases:

<https://developer.android.com/build/shrink-code#keep-code>
<https://www.guardsquare.com/manual/configuration/examples>
<https://gendignoux.com/blog/2022/10/24/rust-library-android.html#shrinking-and-testing-the-release-apk>
