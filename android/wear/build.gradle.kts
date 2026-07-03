plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.fossisawesome.firmium.wear"
    compileSdk = 36

    defaultConfig {
        // Must match the phone app so the system associates the two and the Wearable
        // Data Layer can route between them. The wear APK ships as a separate artifact.
        applicationId = "com.fossisawesome.firmium"
        minSdk = 30
        targetSdk = 36
        versionCode = 25
        versionName = "6.4.1"
    }

    // Mirror the phone app's optional release signing so paired release APKs share a
    // signature — the Data Layer only connects nodes signed with the same key.
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
        debug { isDebuggable = true }
        release {
            if (canSign) signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }

    // logging-interceptor and jspecify both ship this file; same conflict the phone app's
    // build.gradle.kts already excludes.
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
    val composeBom = platform("androidx.compose:compose-bom:2026.05.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material:material-icons-extended")
    debugImplementation("androidx.compose.ui:ui-tooling")

    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.10.0")

    // Keystore-backed credential storage (mirrors the phone app's SecureStorage.kt)
    implementation("androidx.security:security-crypto:1.1.0-alpha06")

    // Compose for Wear OS (classic Material — stable; Material 3 for Wear is still alpha)
    implementation("androidx.wear.compose:compose-material:1.4.1")
    implementation("androidx.wear.compose:compose-foundation:1.4.1")

    // Phone <-> watch communication (Wearable Data Layer)
    implementation("com.google.android.gms:play-services-wearable:19.0.0")

    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")

    // OpenSubsonic API client (Task 5) — same versions as the phone app's build.gradle.kts
    implementation("com.squareup.okhttp3:okhttp:5.4.0")
    implementation("com.squareup.okhttp3:logging-interceptor:5.4.0")
    implementation("com.google.code.gson:gson:2.14.0")

    // Playback engine (Task 5) — same versions as the phone app's build.gradle.kts
    implementation("androidx.media3:media3-exoplayer:1.4.1")
    implementation("androidx.media3:media3-common:1.4.1")
    implementation("androidx.datastore:datastore-preferences:1.2.1")

    // Browse UI navigation — same version as the already-pinned compose-material/compose-foundation
    implementation("androidx.wear.compose:compose-navigation:1.4.1")

    // Cover art (same version as the phone app)
    implementation("io.coil-kt:coil-compose:2.7.0")

    // Search text/voice entry (RemoteInputIntentHelper)
    implementation("androidx.wear:wear-input:1.2.0")
}
