plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.crux.examples.counter.middleware.shared"

    compileSdk {
        version = release(37) {
            minorApiLevel = 1
        }
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
    // module's API.
    api(libs.kotlinx.coroutines.core)
}
