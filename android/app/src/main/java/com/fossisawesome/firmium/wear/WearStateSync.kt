package com.fossisawesome.firmium.wear

import android.graphics.Bitmap
import android.graphics.drawable.BitmapDrawable
import coil.imageLoader
import coil.request.ImageRequest
import coil.request.SuccessResult
import com.fossisawesome.firmium.FirmiumApplication
import com.google.android.gms.tasks.Tasks
import com.google.android.gms.wearable.Asset
import com.google.android.gms.wearable.PutDataMapRequest
import com.google.android.gms.wearable.Wearable
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import java.io.ByteArrayOutputStream

private const val ART_SIZE = 320
private const val ART_QUALITY = 85

// Watches the app-scoped PlaybackController and mirrors a compact now-playing snapshot to the
// paired watch over the Wearable Data Layer. DataClient retains the last item, so the watch shows
// the correct state the instant its UI opens. Album art rides along as a downscaled JPEG Asset.
class WearStateSync(private val app: FirmiumApplication) {

    private val dataClient by lazy { Wearable.getDataClient(app) }
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private var lastArtTrackId: String? = null
    private var lastArtAsset: Asset? = null

    fun start() {
        scope.launch {
            app.playback.state
                .map { s ->
                    val track = s.currentTrack
                    Snapshot(
                        hasTrack = track != null,
                        title = track?.title ?: "",
                        artist = track?.let { it.displayArtist ?: it.artist } ?: "",
                        album = track?.album ?: "",
                        isPlaying = s.playbackState == "playing",
                        volume = s.volume,
                        trackId = track?.id ?: "",
                        coverArt = track?.coverArt,
                    )
                }
                .distinctUntilChanged()
                .collect { push(it) }
        }
    }

    private suspend fun push(snap: Snapshot) {
        val asset = if (snap.hasTrack) artFor(snap) else null
        val request = PutDataMapRequest.create(WearContract.NOW_PLAYING_PATH).apply {
            dataMap.putBoolean(WearContract.KEY_HAS_TRACK, snap.hasTrack)
            dataMap.putString(WearContract.KEY_TITLE, snap.title)
            dataMap.putString(WearContract.KEY_ARTIST, snap.artist)
            dataMap.putString(WearContract.KEY_ALBUM, snap.album)
            dataMap.putBoolean(WearContract.KEY_IS_PLAYING, snap.isPlaying)
            dataMap.putFloat(WearContract.KEY_VOLUME, snap.volume)
            dataMap.putString(WearContract.KEY_TRACK_ID, snap.trackId)
            asset?.let { dataMap.putAsset(WearContract.KEY_ART, it) }
        }.asPutDataRequest().setUrgent()
        try {
            Tasks.await(dataClient.putDataItem(request))
        } catch (_: Exception) {
        }
    }

    // Reuse the Asset across play/pause/volume changes; only re-encode when the track changes.
    private suspend fun artFor(snap: Snapshot): Asset? {
        if (snap.trackId == lastArtTrackId && lastArtAsset != null) return lastArtAsset
        val cover = snap.coverArt ?: return null
        val url = if (cover.startsWith("file://")) cover else app.auth.coverArtUrl(cover, ART_SIZE)
        return try {
            val req = ImageRequest.Builder(app)
                .data(url)
                .size(ART_SIZE)
                .allowHardware(false)
                .build()
            val result = app.imageLoader.execute(req) as? SuccessResult ?: return null
            val bmp = (result.drawable as? BitmapDrawable)?.bitmap ?: return null
            val bytes = ByteArrayOutputStream().use { out ->
                bmp.compress(Bitmap.CompressFormat.JPEG, ART_QUALITY, out)
                out.toByteArray()
            }
            Asset.createFromBytes(bytes).also {
                lastArtTrackId = snap.trackId
                lastArtAsset = it
            }
        } catch (_: Exception) {
            null
        }
    }

    private data class Snapshot(
        val hasTrack: Boolean,
        val title: String,
        val artist: String,
        val album: String,
        val isPlaying: Boolean,
        val volume: Float,
        val trackId: String,
        val coverArt: String?,
    )
}
