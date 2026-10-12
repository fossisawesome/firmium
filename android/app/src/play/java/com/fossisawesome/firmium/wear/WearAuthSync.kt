package com.fossisawesome.firmium.wear

import android.content.Context
import com.google.android.gms.tasks.Tasks
import com.google.android.gms.wearable.PutDataMapRequest
import com.google.android.gms.wearable.Wearable
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

// Pushes the phone's active-account credentials to a paired watch over the Wearable Data
// Layer, so the watch can authenticate against the OpenSubsonic server on its own (standalone
// Wear OS work). DataClient retains the last item, so a watch that's offline when this runs
// catches up the next time it connects — no need to check for a connected node first.
class WearAuthSync(context: Context) {

    private val dataClient = Wearable.getDataClient(context)
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    fun push(serverUrl: String, username: String, password: String) {
        scope.launch {
            val request = PutDataMapRequest.create(WearContract.AUTH_PATH).apply {
                dataMap.putBoolean(WearContract.KEY_HAS_ACCOUNT, true)
                dataMap.putString(WearContract.KEY_SERVER_URL, serverUrl)
                dataMap.putString(WearContract.KEY_USERNAME, username)
                dataMap.putString(WearContract.KEY_PASSWORD, password)
            }.asPutDataRequest().setUrgent()
            try {
                Tasks.await(dataClient.putDataItem(request))
            } catch (e: Exception) {
                android.util.Log.d("WearAuthSync", "putDataItem failed, ignoring", e)
            }
        }
    }

    fun clear() {
        scope.launch {
            val request = PutDataMapRequest.create(WearContract.AUTH_PATH).apply {
                dataMap.putBoolean(WearContract.KEY_HAS_ACCOUNT, false)
            }.asPutDataRequest().setUrgent()
            try {
                Tasks.await(dataClient.putDataItem(request))
            } catch (e: Exception) {
                android.util.Log.d("WearAuthSync", "putDataItem failed, ignoring", e)
            }
        }
    }
}
