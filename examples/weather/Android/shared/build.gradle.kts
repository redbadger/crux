plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.crux.example.weather.shared"

    compileSdk {
        version = release(36)
    }

    defaultConfig {
        minSdk = 28
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
