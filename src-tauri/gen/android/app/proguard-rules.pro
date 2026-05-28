# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# javax.annotation classes are not in the Android SDK but are referenced by
# Google Tink (a transitive dependency of androidx.security:security-crypto).
# R8 treats missing annotation classes as errors by default — suppress them.
-dontwarn javax.annotation.**
-dontwarn javax.annotation.concurrent.**

# Keep Tauri plugin classes so R8 doesn't strip them during minification.
-keep class app.tauri.** { *; }
-keep @app.tauri.annotation.TauriPlugin class * { *; }

# Keep our plugin and service classes.
-keep class com.fossisawesome.firmium.NowPlayingPlugin { *; }
-keep class com.fossisawesome.firmium.NowPlayingService { *; }
-keep class com.fossisawesome.firmium.SecureStoragePlugin { *; }