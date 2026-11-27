plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.crux.examples.counter.middleware.shared"

    compileSdk {
        version = release(36)
    }

    defaultConfig {
        minSdk = 34
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    sourceSets {
        getByName("main") {
            kotlin.directories += "${projectDir}/../generated"
            jniLibs.directories += "${projectDir}/../generated/jniLibs"
        }
    }
}

dependencies {
    // The generated `Core` runs each request on a `CoroutineScope` and
    // publishes the view model as a `StateFlow`, so both appear in this
    // module's API. The typegen's own `build.gradle.kts` declares this, but
    // these sources come in through `srcDirs` instead, so the module has to.
    api(libs.kotlinx.coroutines.core)
}
