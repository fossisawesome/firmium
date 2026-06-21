package com.fossisawesome.firmium.ui.components

import android.Manifest
import android.content.pm.PackageManager
import android.media.audiofx.Visualizer
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin

// Visualizer kinds selectable in Settings and cycled in the player.
enum class VisualizerType(val id: String, val label: String) {
    ORB("orb", "Orb"),
    BARS("bars", "Bars"),
    OSCILLOSCOPE("oscilloscope", "Oscilloscope");

    companion object {
        fun fromId(id: String): VisualizerType = entries.firstOrNull { it.id == id } ?: ORB
    }
}

// Live audio data shared by all visualizers. Populated off the audio thread by a single
// android.media.audiofx.Visualizer; visualizers read whichever field they need.
class VisualizerData {
    var waveform by mutableStateOf(FloatArray(0))   // time-domain, normalized to -1..1
    var magnitudes by mutableStateOf(FloatArray(0)) // per-bin FFT magnitude, 0..1
    var bass by mutableFloatStateOf(0f)             // smoothed low-frequency energy, 0..1
}

// Attaches a Visualizer to ExoPlayer's session and decodes BOTH the waveform and the FFT.
// Deriving bass from real low-frequency FFT bins (not an arbitrary slice of the waveform) is
// what keeps the reaction in sync with the music. No-ops without RECORD_AUDIO or while paused.
@Composable
fun rememberVisualizerData(audioSessionId: Int, isPlaying: Boolean): VisualizerData {
    val context = LocalContext.current
    val data = remember { VisualizerData() }

    DisposableEffect(audioSessionId, isPlaying) {
        if (audioSessionId == 0 || !isPlaying) {
            data.bass = 0f
            data.waveform = FloatArray(0)
            data.magnitudes = FloatArray(0)
            return@DisposableEffect onDispose {}
        }
        val hasRecordAudio = ContextCompat.checkSelfPermission(
            context, Manifest.permission.RECORD_AUDIO
        ) == PackageManager.PERMISSION_GRANTED
        if (!hasRecordAudio) return@DisposableEffect onDispose {}

        val viz = try {
            Visualizer(audioSessionId).apply {
                // Largest capture size for the best frequency/time resolution.
                captureSize = Visualizer.getCaptureSizeRange()[1]
                setDataCaptureListener(object : Visualizer.OnDataCaptureListener {
                    override fun onWaveFormDataCapture(v: Visualizer, wave: ByteArray, sr: Int) {
                        val wf = FloatArray(wave.size)
                        for (i in wave.indices) {
                            wf[i] = ((wave[i].toInt() and 0xFF) - 128) / 128f
                        }
                        data.waveform = wf
                    }

                    override fun onFftDataCapture(v: Visualizer, fft: ByteArray, sr: Int) {
                        // Android FFT layout: [Re0, Re(n/2), Re1, Im1, Re2, Im2, ...].
                        val bins = fft.size / 2
                        if (bins <= 0) return
                        val mags = FloatArray(bins)
                        mags[0] = (kotlin.math.abs(fft[0].toInt()) / 128f).coerceIn(0f, 1f)
                        var bassSum = 0f
                        var bassCount = 0
                        val bassCutoff = max(1, bins / 8)
                        for (k in 1 until bins) {
                            val re = fft[2 * k].toFloat()
                            val im = fft[2 * k + 1].toFloat()
                            val mag = (hypot(re, im) / 128f).coerceIn(0f, 1f)
                            mags[k] = mag
                            if (k <= bassCutoff) { bassSum += mag; bassCount++ }
                        }
                        data.magnitudes = mags
                        val target = if (bassCount > 0) (bassSum / bassCount * 1.8f).coerceIn(0f, 1f) else 0f
                        // Fast attack, slow decay — reads as "on the beat" rather than mushy.
                        data.bass = if (target > data.bass) target else data.bass * 0.86f + target * 0.14f
                    }
                }, Visualizer.getMaxCaptureRate(), true, true)
                enabled = true
            }
        } catch (_: Exception) { null }

        onDispose { try { viz?.enabled = false; viz?.release() } catch (_: Exception) {} }
    }
    return data
}

// Shared 3-stop color ramp (matches the orb's palette cycling).
internal fun vizPaletteColor(p: OrbPalette, phase: Float): Color {
    val t = ((phase % 1f) + 1f) % 1f
    return when {
        t < 0.33f -> lerp(p.primary, p.secondary, t / 0.33f)
        t < 0.66f -> lerp(p.secondary, p.tertiary, (t - 0.33f) / 0.33f)
        else      -> lerp(p.tertiary, p.primary, (t - 0.66f) / 0.34f)
    }
}

// Dispatches to the selected visualizer.
@Composable
fun VisualizerView(
    type: VisualizerType,
    audioSessionId: Int,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    when (type) {
        VisualizerType.ORB -> MusicOrb(audioSessionId, palette, isPlaying, modifier)
        VisualizerType.BARS -> BarVisualizer(audioSessionId, palette, isPlaying, modifier)
        VisualizerType.OSCILLOSCOPE -> CircularOscilloscope(audioSessionId, palette, isPlaying, modifier)
    }
}

private const val BAR_COUNT = 10

// Classic frequency-bar visualizer. Bars use a log-ish band mapping so bass doesn't dominate.
@Composable
fun BarVisualizer(
    audioSessionId: Int,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    val data = rememberVisualizerData(audioSessionId, isPlaying)
    val smoothed = remember { FloatArray(BAR_COUNT) }
    val infinite = rememberInfiniteTransition(label = "bars")
    val clock by infinite.animateFloat(
        initialValue = 0f, targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(8000, easing = LinearEasing)),
        label = "barClock",
    )

    Canvas(modifier = modifier.fillMaxSize()) {
        val mags = data.magnitudes
        val n = BAR_COUNT
        val slot = size.width / n
        val barW = slot * 0.7f
        val bins = mags.size
        for (i in 0 until n) {
            val target = if (bins == 0) 0f else {
                // Map bar i to a frequency band over the lower ~75% of the spectrum (log spacing).
                val loF = (i.toFloat() / n); val hiF = ((i + 1f) / n)
                val lo = (loF.pow(1.5f) * bins * 0.9f).toInt().coerceIn(0, bins - 1)
                val hi = (hiF.pow(1.5f) * bins * 0.9f).toInt().coerceIn(lo + 1, bins)
                var m = 0f
                for (k in lo until hi) m = max(m, mags[k])
                (m * 1.1f).coerceIn(0f, 1f)
            }
            smoothed[i] = if (target > smoothed[i]) target else smoothed[i] * 0.82f + target * 0.18f
            val h = (smoothed[i] * size.height).coerceIn(2f, size.height)
            drawRoundRect(
                color = vizPaletteColor(palette, clock + i.toFloat() / n),
                topLeft = Offset(i * slot + (slot - barW) / 2f, size.height - h),
                size = Size(barW, h),
                cornerRadius = CornerRadius(barW * 0.35f),
            )
        }
    }
}

// Oscilloscope wrapped into a circle: the waveform modulates the radius of a closed ring.
@Composable
fun CircularOscilloscope(
    audioSessionId: Int,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    val data = rememberVisualizerData(audioSessionId, isPlaying)
    val infinite = rememberInfiniteTransition(label = "scope")
    val clock by infinite.animateFloat(
        initialValue = 0f, targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(8000, easing = LinearEasing)),
        label = "scopeClock",
    )

    Canvas(modifier = modifier.fillMaxSize()) {
        val cx = size.width / 2f
        val cy = size.height / 2f
        val maxR = min(size.width, size.height) / 2f
        val baseR = maxR * 0.58f
        val amp = maxR * 0.34f
        val wave = data.waveform

        // Faint guide circle so it reads as a scope even in near-silence.
        drawCircle(
            color = vizPaletteColor(palette, clock).copy(alpha = 0.18f),
            radius = baseR, center = Offset(cx, cy),
            style = Stroke(width = 1.5f),
        )

        if (wave.isNotEmpty()) {
            val points = 200
            val path = Path()
            for (j in 0..points) {
                val sample = wave[(j % points) * wave.size / points]
                val angle = (j.toFloat() / points) * 2f * PI.toFloat()
                val r = baseR + sample * amp
                val x = cx + cos(angle) * r
                val y = cy + sin(angle) * r
                if (j == 0) path.moveTo(x, y) else path.lineTo(x, y)
            }
            path.close()
            // Soft outer pass + crisp inner pass for a neon look.
            drawPath(path, color = vizPaletteColor(palette, clock + 0.5f).copy(alpha = 0.35f),
                style = Stroke(width = 7f))
            drawPath(path, color = vizPaletteColor(palette, clock),
                style = Stroke(width = 2.5f))
        }
    }
}
