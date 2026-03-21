import com.android.build.gradle.tasks.MergeSourceSetFolders
import com.nishtahir.CargoBuildTask
import com.nishtahir.CargoExtension
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.rust.android)
}

android {
    namespace = "com.crux.examples.notes.shared"

    compileSdk {
        version = release(36)
    }

    ndkVersion = "29.0.14206865"

    defaultConfig {
        minSdk = 34
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlin {
        compilerOptions {
            jvmTarget = JvmTarget.JVM_11
        }
    }

    sourceSets {
        getByName("main") {
            kotlin.srcDirs("${projectDir}/../generated")
        }
    }
}

dependencies {
    implementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }
}

extensions.configure<CargoExtension>("cargo") {
    module = "../.."
    libname = "shared"
    profile = "debug"
    targets = listOf("arm", "arm64", "x86", "x86_64")
    extraCargoBuildArguments = listOf("--package", "shared", "--features", "uniffi")
    cargoCommand = System.getProperty("user.home") + "/.cargo/bin/cargo"
    rustcCommand = System.getProperty("user.home") + "/.cargo/bin/rustc"
    pythonCommand = "python3"
}

afterEvaluate {
    android.libraryVariants.configureEach {
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
