package com.fossisawesome.firmium.wear

import android.content.Context
import com.google.android.gms.tasks.Tasks
import com.google.android.gms.wearable.DataClient
import com.google.android.gms.wearable.DataEvent
import com.google.android.gms.wearable.DataMap
import com.google.android.gms.wearable.DataMapItem
import com.google.android.gms.wearable.Wearable
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

// Watch-side receiver for the active-account credentials the phone pushes over the Wearable
// Data Layer. Stores into WatchSecureStorage so later standalone-auth work (API client,
// playback) can read credentials without the phone present.
class WearAuthClient(context: Context) {

    private val dataClient = Wearable.getDataClient(context)
    private val storage = WatchSecureStorage(context.applicationContext)
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private val listener = DataClient.OnDataChangedListener { events ->
        for (event in events) {
            if (event.type == DataEvent.TYPE_CHANGED &&
                event.dataItem.uri.path == WearContract.AUTH_PATH
            ) {
                applyDataMap(DataMapItem.fromDataItem(event.dataItem).dataMap)
            }
        }
    }

    fun start() {
        dataClient.addListener(listener)
        // DataClient retains the last item, so a freshly reopened/rebooted watch catches up
        // without the phone needing to push again.
        scope.launch { loadCurrent() }
    }

    fun stop() {
        dataClient.removeListener(listener)
    }

    private fun loadCurrent() {
        try {
            val buffer = Tasks.await(dataClient.dataItems)
            try {
                for (item in buffer) {
                    if (item.uri.path == WearContract.AUTH_PATH) {
                        applyDataMap(DataMapItem.fromDataItem(item).dataMap)
                    }
                }
            } finally {
                buffer.release()
            }
        } catch (_: Exception) {
        }
    }

    private fun applyDataMap(map: DataMap) {
        if (!map.getBoolean(WearContract.KEY_HAS_ACCOUNT)) {
            storage.clear()
            return
        }
        val serverUrl = map.getString(WearContract.KEY_SERVER_URL) ?: return
        val username = map.getString(WearContract.KEY_USERNAME) ?: return
        val password = map.getString(WearContract.KEY_PASSWORD) ?: return
        storage.save(serverUrl, username, password)
    }
}
