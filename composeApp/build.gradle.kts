import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.ExperimentalKotlinGradlePluginApi
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import gobley.gradle.GobleyHost
import gobley.gradle.cargo.dsl.*

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.composeHotReload)

    id("dev.gobley.cargo") version "0.2.0"
    id("dev.gobley.rust") version "0.2.0"
    id("dev.gobley.uniffi") version "0.2.0"
    kotlin("plugin.atomicfu") version libs.versions.kotlin
}

kotlin {
    androidTarget {
        @OptIn(ExperimentalKotlinGradlePluginApi::class)
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_11)
        }
    }

    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.binaries.framework {
            baseName = "ComposeApp"
            isStatic = true
        }
    }

    jvm("desktop")

    sourceSets {
        val desktopMain by getting

        androidMain.dependencies {
            implementation(compose.preview)
            implementation(libs.androidx.activity.compose)

            // QR scanner
            implementation("com.google.android.gms:play-services-code-scanner:16.1.0")
        }
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(compose.components.uiToolingPreview)
            implementation(libs.androidx.lifecycle.viewmodel)
            implementation(libs.androidx.lifecycle.runtimeCompose)
            implementation(libs.androidx.lifecycle.viewmodel.compose)

            // bottom sheet
            implementation("com.composables:core:1.36.1")

            // QR generator
            implementation("io.github.alexzhirkevich:qrose:1.0.1")

            // navigation
            implementation("org.jetbrains.androidx.navigation:navigation-compose:2.9.0-beta03")

            // multiplatform settings
            implementation("com.russhwolf:multiplatform-settings-no-arg:1.3.0")
            implementation("com.russhwolf:multiplatform-settings-make-observable:1.3.0")
            implementation("com.russhwolf:multiplatform-settings-coroutines:1.3.0")
        }
        commonTest.dependencies {
            implementation(libs.kotlin.test)
        }
        desktopMain.dependencies {
            implementation(compose.desktop.currentOs)
            implementation(libs.kotlinx.coroutinesSwing)
        }
    }
}

android {
    namespace = "zip.meows.musicopy"
    compileSdk = libs.versions.android.compileSdk.get().toInt()

    defaultConfig {
        applicationId = "zip.meows.musicopy"
        minSdk = libs.versions.android.minSdk.get().toInt()
        targetSdk = libs.versions.android.targetSdk.get().toInt()
        versionCode = 1
        versionName = "1.0"
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

dependencies {
    debugImplementation(compose.uiTooling)
}

compose.desktop {
    application {
        mainClass = "zip.meows.musicopy.MainKt"

        nativeDistributions {
            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)
            packageName = "zip.meows.musicopy"
            packageVersion = "1.0.0"
        }
    }
}

// HACK: disable rustup target add tasks
project.gradle.taskGraph.whenReady {
    project.tasks.forEach { task ->
        if (task.name.contains("rustUpTargetAdd")) {
            task.enabled = false
        }
    }
}

cargo {
    packageDirectory = layout.projectDirectory.dir("../crates/musicopy")

    // build desktop for the host target only
    builds.jvm {
        embedRustLibrary = (rustTarget == GobleyHost.current.rustTarget)
    }
}
