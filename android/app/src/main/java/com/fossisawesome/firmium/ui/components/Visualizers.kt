package com.fossisawesome.firmium.ui.components

import com.fossisawesome.firmium.audio.VisualizerAudioProcessor
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
import kotlin.math.PI
import kotlin.math.cos
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

// Live audio data shared by all visualizers. Populated off the audio thread by a
// VisualizerAudioProcessor tapping ExoPlayer's own PCM pipeline; visualizers read whichever
// field they need.
class VisualizerData {
    var waveform by mutableStateOf(FloatArray(0))   // time-domain, normalized to -1..1
    var magnitudes by mutableStateOf(FloatArray(0)) // per-bin FFT magnitude, 0..1
    var bass by mutableFloatStateOf(0f)             // smoothed low-frequency energy, 0..1
}

// Subscribes to the current player's VisualizerAudioProcessor tap. Deriving bass from real
// low-frequency FFT bins (not an arbitrary slice of the waveform) is what keeps the reaction in
// sync with the music. No system Visualizer effect involved, so no RECORD_AUDIO permission and
// no dependency on session-id timing — the processor sits directly in ExoPlayer's audio pipeline.
@Composable
fun rememberVisualizerData(visualizerProcessor: VisualizerAudioProcessor?, isPlaying: Boolean): VisualizerData {
    val data = remember { VisualizerData() }

    DisposableEffect(visualizerProcessor, isPlaying) {
        if (visualizerProcessor == null || !isPlaying) {
            data.bass = 0f
            data.waveform = FloatArray(0)
            data.magnitudes = FloatArray(0)
            return@DisposableEffect onDispose {}
        }

        visualizerProcessor.onData = { waveform, magnitudes, bass ->
            data.waveform = waveform
            data.magnitudes = magnitudes
            data.bass = bass
        }

        onDispose { visualizerProcessor.onData = null }
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
    visualizerProcessor: VisualizerAudioProcessor?,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    when (type) {
        VisualizerType.ORB -> MusicOrb(visualizerProcessor, palette, isPlaying, modifier)
        VisualizerType.BARS -> BarVisualizer(visualizerProcessor, palette, isPlaying, modifier)
        VisualizerType.OSCILLOSCOPE -> CircularOscilloscope(visualizerProcessor, palette, isPlaying, modifier)
    }
}

private const val BAR_COUNT = 10

// Classic frequency-bar visualizer. Bars use a log-ish band mapping so bass doesn't dominate.
@Composable
fun BarVisualizer(
    visualizerProcessor: VisualizerAudioProcessor?,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    val data = rememberVisualizerData(visualizerProcessor, isPlaying)
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
    visualizerProcessor: VisualizerAudioProcessor?,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    val data = rememberVisualizerData(visualizerProcessor, isPlaying)
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
