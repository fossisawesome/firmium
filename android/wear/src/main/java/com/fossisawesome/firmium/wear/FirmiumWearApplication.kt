package com.fossisawesome.firmium.wear

import android.app.Application
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.WatchAuthManager

// Watch module's DI container — mirrors the phone app's FirmiumApplication. Needed because
// auth state and the API client must be shared app-wide (a future playback service won't
// have an Activity to own them), not re-created per-Activity like WearPlaybackClient is.
class FirmiumWearApplication : Application() {
    val secureStorage by lazy { WatchSecureStorage(this) }
    val authManager by lazy { WatchAuthManager(secureStorage) }
    val api by lazy { ApiClient(authManager) }
}
