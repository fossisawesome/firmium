package com.fossisawesome.firmium.wear

import android.content.Context

// F-Droid build (-Pfoss): no watch sync, see WearStateSync.
class WearAuthSync(@Suppress("UNUSED_PARAMETER") context: Context) {
    fun push(serverUrl: String, username: String, password: String) {}
    fun clear() {}
}
