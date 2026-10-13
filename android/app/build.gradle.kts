plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.google.devtools.ksp")
}

// ./gradlew assembleRelease -Pfoss builds the F-Droid variant without Google libraries.
val isFoss = providers.gradleProperty("foss").isPresent

android {
    namespace = "com.fossisawesome.firmium"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.fossisawesome.firmium"
        minSdk = 26
        targetSdk = 36
        versionCode = 36
        versionName = "9.0.2"
        manifestPlaceholders["usesCleartextTraffic"] = "false"
    }

    val signingKeyPath = System.getenv("ANDROID_SIGNING_KEY_PATH")
    val signingKeyAlias = System.getenv("ANDROID_SIGNING_KEY_ALIAS")
    val signingStorePassword = System.getenv("ANDROID_SIGNING_STORE_PASSWORD")
    val signingKeyPassword = System.getenv("ANDROID_SIGNING_KEY_PASSWORD")
    val canSign = listOf(signingKeyPath, signingKeyAlias, signingStorePassword, signingKeyPassword).all { it != null }

    if (canSign) {
        signingConfigs {
            create("release") {
                storeFile = file(signingKeyPath!!)
                storePassword = signingStorePassword
                keyAlias = signingKeyAlias
                keyPassword = signingKeyPassword
            }
        }
    }

    buildTypes {
        debug {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
        }
        release {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            if (canSign) signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    sourceSets["main"].kotlin.directories.add(if (isFoss) "src/foss/java" else "src/play/java")

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    buildFeatures { compose = true; buildConfig = true }

    lint {
        disable += "InvalidFragmentVersionForActivityResult"
        // Voice search ("play some jazz" from the Google Assistant remote) needs a real
        // MEDIA_PLAY_FROM_SEARCH handler wired to playback — out of scope for TV Phase 1
        // (browse + playback only). Revisit when voice search is actually implemented.
        disable += "MissingIntentFilterForMediaSearch"
    }

    packaging {
        resources {
            excludes += "META-INF/versions/9/OSGI-INF/MANIFEST.MF"
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_11)
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.09.00")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.foundation:foundation")
    debugImplementation("androidx.compose.ui:ui-tooling")

    implementation("androidx.navigation:navigation-compose:2.10.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.11.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.11.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.core:core-splashscreen:1.2.0")

    // Android TV: D-pad-focusable Compose components for the leanback UI
    implementation("androidx.tv:tv-foundation:1.0.0")
    implementation("androidx.tv:tv-material:1.1.0")

    // HTTP + JSON
    implementation("com.squareup.okhttp3:okhttp:5.5.0")
    implementation("com.squareup.okhttp3:logging-interceptor:5.5.0")
    implementation("com.google.code.gson:gson:2.14.0")

    // Image loading
    implementation("io.coil-kt:coil-compose:2.7.0")
    // Dominant-color extraction for the full-screen player background gradient
    implementation("androidx.palette:palette-ktx:1.0.0")

    // Audio
    implementation("androidx.media3:media3-exoplayer:1.11.1")
    implementation("androidx.media3:media3-common:1.11.1")
    implementation("androidx.media3:media3-datasource-okhttp:1.11.1")
    implementation("androidx.media:media:1.8.0")

    // Wear OS companion: phone <-> watch communication (Wearable Data Layer).
    // Skipped with -Pfoss (F-Droid), where src/foss supplies no-op stubs instead.
    if (!isFoss) implementation("com.google.android.gms:play-services-wearable:20.0.1")

    // Secure storage
    implementation("androidx.security:security-crypto:1.1.0")

    // DataStore for non-sensitive preferences (server URL, settings)
    implementation("androidx.datastore:datastore-preferences:1.2.1")

    // Room: local play-history store
    implementation("androidx.room:room-runtime:2.8.5")
    implementation("androidx.room:room-ktx:2.8.5")
    ksp("androidx.room:room-compiler:2.8.5")

    // Adaptive layout: WindowSizeClass + FoldingFeature for foldable / large screen support
    implementation("androidx.window:window:1.5.1")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
    testImplementation("org.mockito.kotlin:mockito-kotlin:6.4.0")
    testImplementation("org.mockito:mockito-core:5.24.0")
}
