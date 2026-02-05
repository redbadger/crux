import com.android.build.gradle.tasks.MergeSourceSetFolders
import com.nishtahir.CargoBuildTask
import com.nishtahir.CargoExtension

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.rust.android)
}

android {
    namespace = "com.crux.example.counter.shared"
    compileSdk = 36

    ndkVersion = "29.0.14206865"

    defaultConfig {
        minSdk = 34

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
    sourceSets {
        getByName("main") {
            kotlin.srcDirs("../generated/app")
            kotlin.srcDirs("../generated/sse")
            java.srcDirs("../generated/serde")
        }
    }
}

dependencies {
    implementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.material)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
}

apply(plugin = "org.mozilla.rust-android-gradle.rust-android")

// Try this approach instead of configure<>
extensions.configure<CargoExtension>("cargo") {
    module = "../.."
    libname = "shared"
    profile = "debug"
    // these are the four recommended targets for Android that will ensure your library works on all mainline android devices
    // make sure you have included the rust toolchain for each of these targets: \
    // `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`
    targets = listOf("arm", "arm64", "x86", "x86_64")
    extraCargoBuildArguments = listOf("--package", "shared", "--features", "uniffi")
    cargoCommand = System.getProperty("user.home") + "/.cargo/bin/cargo"
    rustcCommand = System.getProperty("user.home") + "/.cargo/bin/rustc"
    pythonCommand = "python3"
}

afterEvaluate {
    // The `cargoBuild` task isn't available until after evaluation.
    android.libraryVariants.configureEach {
        var productFlavor = ""
        productFlavors.forEach { flavor ->
            productFlavor += flavor.name.replaceFirstChar { char -> char.uppercaseChar() }
        }
        val buildType = buildType.name.replaceFirstChar { char -> char.uppercaseChar() }

        tasks.named("generate${productFlavor}${buildType}Assets") {
            dependsOn(tasks.named("cargoBuild"))
        }

        // The below dependsOn is needed till https://github.com/mozilla/rust-android-gradle/issues/85 is resolved this fix was got from #118
        tasks.withType<CargoBuildTask>().forEach { buildTask ->
            tasks.withType<MergeSourceSetFolders>().configureEach {
                inputs.dir(
                    File(
                        File(layout.buildDirectory.asFile.get(), "rustJniLibs"),
                        buildTask.toolchain?.folder!!
                    )
                )
                dependsOn(buildTask)
            }
        }
    }
}

// The below dependsOn is needed till https://github.com/mozilla/rust-android-gradle/issues/85 is resolved this fix was got from #118
tasks.matching { it.name.matches(Regex("merge.*JniLibFolders")) }.configureEach {
    inputs.dir(File(layout.buildDirectory.asFile.get(), "rustJniLibs/android"))
    dependsOn("cargoBuild")
}
