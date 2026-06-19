package com.fossisawesome.firmium.audio

import android.media.audiofx.BassBoost
import android.media.audiofx.Equalizer
import android.util.Log
import java.util.concurrent.ConcurrentHashMap
import kotlin.math.abs
import kotlin.math.roundToInt

private const val TAG = "FirmiumEq"

/**
 * Manages [android.media.audiofx.Equalizer] + [BassBoost] effects attached to each
 * active ExoPlayer audio session.
 *
 * The system Equalizer exposes a fixed set of hardware bands, so both modes are
 * lossy mappings of the app's logical bands:
 *  - Graphic: each system band is set to the gain of the nearest logical band.
 *  - Parametric: same nearest-band mapping; Q is not representable in the system
 *    EQ and is ignored (documented approximation).
 * BassBoost is driven by the lowest band's positive gain for extra low-end punch.
 */
class EqualizerController {

    data class Band(val freq: Float, val gain: Float, val q: Float?)
    data class Config(val enabled: Boolean, val mode: String, val bands: List<Band>)

    @Volatile
    private var config = Config(enabled = false, mode = "graphic", bands = emptyList())

    private val attached = ConcurrentHashMap<Int, Effects>()

    private class Effects(val equalizer: Equalizer, val bassBoost: BassBoost) {
        fun release() {
            runCatching { equalizer.release() }
            runCatching { bassBoost.release() }
        }
    }

    /** Attach effects to a player's audio session (no-op for the unset session 0). */
    fun attach(sessionId: Int) {
        if (sessionId == 0) return
        detach(sessionId)
        try {
            val effects = Effects(Equalizer(0, sessionId), BassBoost(0, sessionId))
            attached[sessionId] = effects
            applyTo(effects)
        } catch (e: Exception) {
            Log.w(TAG, "Failed to attach EQ to session $sessionId", e)
        }
    }

    fun detach(sessionId: Int) {
        attached.remove(sessionId)?.release()
    }

    /** Update the active config and push it to every attached session. */
    fun setConfig(newConfig: Config) {
        config = newConfig
        attached.values.forEach { applyTo(it) }
    }

    private fun gainAt(freqHz: Float): Float {
        val bands = config.bands
        if (bands.isEmpty()) return 0f
        return bands.minByOrNull { abs(it.freq - freqHz) }?.gain ?: 0f
    }

    private fun applyTo(effects: Effects) {
        val cfg = config
        try {
            effects.equalizer.enabled = cfg.enabled
            effects.bassBoost.enabled = cfg.enabled
            if (!cfg.enabled) return

            val range = effects.equalizer.bandLevelRange // [min, max] in millibels
            val minLevel = range[0].toInt()
            val maxLevel = range[1].toInt()
            val numBands = effects.equalizer.numberOfBands.toInt()
            for (b in 0 until numBands) {
                val centerHz = effects.equalizer.getCenterFreq(b.toShort()) / 1000f // milliHz → Hz
                val targetMb = (gainAt(centerHz) * 100f).roundToInt() // dB → millibels
                effects.equalizer.setBandLevel(b.toShort(), targetMb.coerceIn(minLevel, maxLevel).toShort())
            }

            // BassBoost from the lowest configured band's positive gain (0..1000 strength).
            val lowGain = cfg.bands.minByOrNull { it.freq }?.gain ?: 0f
            val strength = ((lowGain / 12f) * 1000f).roundToInt().coerceIn(0, 1000)
            if (effects.bassBoost.strengthSupported) {
                effects.bassBoost.setStrength(strength.toShort())
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to apply EQ config", e)
        }
    }
}
