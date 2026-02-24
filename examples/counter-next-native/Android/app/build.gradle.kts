import com.android.build.gradle.tasks.MergeSourceSetFolders
import com.nishtahir.CargoBuildTask
import com.nishtahir.CargoExtension

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.rust.android)
}

android {
    namespace = "com.crux.example.counter"
    compileSdk = 36

    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "com.crux.example.counter"
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
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
    buildFeatures {
        compose = true
    }
    sourceSets {
        getByName("main") {
            // UniFFI-generated Kotlin bindings
            kotlin.srcDirs("../generated/app")
        }
    }
}

dependencies {
    // JNA for UniFFI native bindings
    implementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }

    implementation(libs.lifecycle.viewmodel.compose)
    implementation(libs.ktor.client)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)
    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}

extensions.configure<CargoExtension>("cargo") {
    module = "../.."
    libname = "shared"
    profile = "debug"
    targets = listOf("arm", "arm64", "x86", "x86_64")
    extraCargoBuildArguments = listOf("--package", "shared", "--features", "native_bridge")
    cargoCommand = System.getProperty("user.home") + "/.cargo/bin/cargo"
    rustcCommand = System.getProperty("user.home") + "/.cargo/bin/rustc"
    pythonCommand = "python3"
}

afterEvaluate {
    android.applicationVariants.configureEach {
        var productFlavor = ""
        productFlavors.forEach { flavor ->
            productFlavor += flavor.name.replaceFirstChar { char -> char.uppercaseChar() }
        }
        val buildType = buildType.name.replaceFirstChar { char -> char.uppercaseChar() }

        tasks.named("generate${productFlavor}${buildType}Assets") {
            dependsOn(tasks.named("cargoBuild"))
        }

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

tasks.matching { it.name.matches(Regex("merge.*JniLibFolders")) }.configureEach {
    inputs.dir(File(layout.buildDirectory.asFile.get(), "rustJniLibs/android"))
    dependsOn("cargoBuild")
}
