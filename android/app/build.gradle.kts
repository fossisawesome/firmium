plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.fossisawesome.firmium"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.fossisawesome.firmium"
        minSdk = 26
        targetSdk = 36
        versionCode = 3
        versionName = "4.0.2"
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
            if (canSign) signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    kotlinOptions { jvmTarget = "11" }
    buildFeatures { compose = true; buildConfig = true }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.05.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.foundation:foundation")
    debugImplementation("androidx.compose.ui:ui-tooling")

    implementation("androidx.navigation:navigation-compose:2.9.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.9.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.0")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.core:core-splashscreen:1.0.1")

    // HTTP + JSON
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.okhttp3:logging-interceptor:4.12.0")
    implementation("com.google.code.gson:gson:2.11.0")

    // Image loading
    implementation("io.coil-kt:coil-compose:2.7.0")
    // Dominant-color extraction for the full-screen player background gradient
    implementation("androidx.palette:palette-ktx:1.0.0")

    // Audio
    implementation("androidx.media3:media3-exoplayer:1.4.1")
    implementation("androidx.media3:media3-common:1.4.1")
    implementation("androidx.media:media:1.7.0")

    // Secure storage
    implementation("androidx.security:security-crypto:1.1.0-alpha06")

    // DataStore for non-sensitive preferences (server URL, settings)
    implementation("androidx.datastore:datastore-preferences:1.1.2")

    // Adaptive layout: WindowSizeClass + FoldingFeature for foldable / large screen support
    implementation("androidx.window:window:1.3.0")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")

    testImplementation("junit:junit:4.13.2")
}
