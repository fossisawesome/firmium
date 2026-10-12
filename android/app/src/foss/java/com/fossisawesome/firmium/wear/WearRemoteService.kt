package com.fossisawesome.firmium.wear

import android.app.Service
import android.content.Intent
import android.os.IBinder

// F-Droid build (-Pfoss): placeholder so the manifest's service entry resolves; nothing binds to it.
class WearRemoteService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null
}
