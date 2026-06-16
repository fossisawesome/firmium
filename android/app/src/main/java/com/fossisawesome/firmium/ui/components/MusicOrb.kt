package com.fossisawesome.firmium.ui.components

import android.media.audiofx.Visualizer
import androidx.compose.animation.core.FastOutSlowInEasing
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
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.lerp
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

private const val PARTICLE_COUNT = 28
private const val RING_COUNT = 3

private fun lerpColor(a: Color, b: Color, t: Float): Color =
    lerp(a, b, t.coerceIn(0f, 1f))

private fun paletteColor(p: OrbPalette, phase: Float): Color {
    val t = ((phase % 1f) + 1f) % 1f
    return when {
        t < 0.33f -> lerpColor(p.primary, p.secondary, t / 0.33f)
        t < 0.66f -> lerpColor(p.secondary, p.tertiary, (t - 0.33f) / 0.33f)
        else      -> lerpColor(p.tertiary, p.primary,   (t - 0.66f) / 0.34f)
    }
}

// NCS-style audio-reactive orb visualizer.
// Attaches to ExoPlayer's audio session via android.media.audiofx.Visualizer to get real
// waveform data; derives a bass amplitude from the low-frequency half of each capture.
// Falls back to a gentle breathe animation when audioSessionId == 0 or Visualizer fails.
@Composable
fun MusicOrb(
    audioSessionId: Int,
    palette: OrbPalette,
    isPlaying: Boolean,
    modifier: Modifier = Modifier,
) {
    var bass by remember { mutableFloatStateOf(0f) }
    var smoothBass by remember { mutableFloatStateOf(0f) }

    DisposableEffect(audioSessionId) {
        if (audioSessionId == 0) return@DisposableEffect onDispose {}
        val viz = try {
            Visualizer(audioSessionId).apply {
                captureSize = Visualizer.getCaptureSizeRange()[0]
                setDataCaptureListener(object : Visualizer.OnDataCaptureListener {
                    override fun onWaveFormDataCapture(v: Visualizer, wave: ByteArray, sr: Int) {
                        val half = wave.size / 2
                        var sum = 0f
                        for (i in 0 until half) {
                            sum += abs((wave[i].toInt() and 0xFF) - 128).toFloat()
                        }
                        bass = (sum / (half * 128f)).coerceIn(0f, 1f)
                    }
                    override fun onFftDataCapture(v: Visualizer, fft: ByteArray, sr: Int) {}
                }, Visualizer.getMaxCaptureRate() / 2, true, false)
                enabled = true
            }
        } catch (_: Exception) { null }

        onDispose {
            try { viz?.enabled = false; viz?.release() } catch (_: Exception) {}
        }
    }

    val infiniteTransition = rememberInfiniteTransition(label = "orb")

    // Main clock drives ring expansion and wisp rotation (8 s period).
    val clock by infiniteTransition.animateFloat(
        initialValue = 0f, targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(8000, easing = LinearEasing)),
        label = "clock",
    )
    // Slower breathe for the orb core (2.4 s period).
    val breathe by infiniteTransition.animateFloat(
        initialValue = 0f, targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(2400, easing = FastOutSlowInEasing)),
        label = "breathe",
    )

    // Fixed particle descriptors: base angle, speed factor, lifetime phase offset.
    val particles = remember {
        (0 until PARTICLE_COUNT).map { i ->
            Triple(
                (i.toFloat() / PARTICLE_COUNT) * 2f * PI.toFloat(),
                0.3f + (i % 7) * 0.1f,
                i.toFloat() / PARTICLE_COUNT,
            )
        }
    }

    Canvas(modifier = modifier.fillMaxSize()) {
        // Lerp smoothBass toward the raw value — prevents jarring jumps on beat drops.
        smoothBass = smoothBass + (bass - smoothBass) * 0.25f

        val cx = size.width / 2f
        val cy = size.height / 2f
        val maxR = min(size.width, size.height) / 2f

        val breatheFrac = (sin(breathe * 2f * PI.toFloat()) * 0.5f + 0.5f)
        val baseR = maxR * (0.28f + breatheFrac * 0.08f)
        // Scale orb radius by up to +55% on bass hits.
        val orbR = baseR * (1f + smoothBass * 0.55f)

        val coreColor  = paletteColor(palette, clock)
        val ringColor1 = paletteColor(palette, clock + 0.33f)
        val ringColor2 = paletteColor(palette, clock + 0.55f)
        val wispColor0 = paletteColor(palette, clock + 0.17f)
        val wispColor1 = paletteColor(palette, clock + 0.50f)
        val particleA  = paletteColor(palette, clock + 0.10f)
        val particleB  = paletteColor(palette, clock + 0.40f)
        val particleC  = paletteColor(palette, clock + 0.70f)

        // ── Core glow: 4 layered radial gradients to fake a soft-blur bloom ────
        for (layer in 3 downTo 0) {
            val factor = layer / 3f
            val alpha = 0.12f + factor * 0.25f
            val r = orbR * (1.8f - factor * 0.8f)
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(
                        coreColor.copy(alpha = alpha),
                        coreColor.copy(alpha = 0f),
                    ),
                    center = Offset(cx, cy),
                    radius = r.coerceAtLeast(1f),
                ),
                radius = r.coerceAtLeast(1f),
                center = Offset(cx, cy),
            )
        }
        // Bright solid core with white hotspot at center.
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(
                    Color.White.copy(alpha = 0.85f),
                    coreColor.copy(alpha = 0.9f),
                    coreColor.copy(alpha = 0f),
                ),
                center = Offset(cx, cy),
                radius = orbR.coerceAtLeast(1f),
            ),
            radius = orbR.coerceAtLeast(1f),
            center = Offset(cx, cy),
        )

        // ── Concentric expanding rings (staggered phase) ─────────────────────
        for (i in 0 until RING_COUNT) {
            val phase = (clock + i.toFloat() / RING_COUNT) % 1f
            val ringR = orbR * (1.1f + phase * 2.2f)
            val ringAlpha = (1f - phase) * (0.4f + smoothBass * 0.4f)
            val strokeW = (3f - phase * 2.5f).coerceAtLeast(0.5f)
            val ringColor = if (i % 2 == 0) ringColor1 else ringColor2
            drawCircle(
                color = ringColor.copy(alpha = ringAlpha),
                radius = ringR.coerceAtLeast(1f),
                center = Offset(cx, cy),
                style = Stroke(width = strokeW),
            )
        }

        // ── 4 orbiting energy wisps ───────────────────────────────────────────
        for (w in 0 until 4) {
            val angle = clock * 2f * PI.toFloat() + w * (PI.toFloat() / 2f)
            val orbitR = orbR * (1.35f + sin(breathe * PI.toFloat() + w) * 0.15f)
            val wx = cx + cos(angle) * orbitR
            val wy = cy + sin(angle) * orbitR
            val wispR = (orbR * (0.18f + smoothBass * 0.12f)).coerceAtLeast(1f)
            val wispColor = if (w % 2 == 0) wispColor0 else wispColor1
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(wispColor.copy(alpha = 0.7f), wispColor.copy(alpha = 0f)),
                    center = Offset(wx, wy),
                    radius = wispR,
                ),
                radius = wispR,
                center = Offset(wx, wy),
            )
        }

        // ── Particle field ────────────────────────────────────────────────────
        for ((baseAngle, speed, phaseOffset) in particles) {
            val age = (clock + phaseOffset) % 1f
            val pAngle = baseAngle + clock * 0.8f
            val pDist = orbR * (0.9f + age * 1.8f * (0.6f + smoothBass * 0.8f) * speed)
            val pAlpha = ((1f - age) * 0.7f).coerceAtLeast(0f)
            val pRadius = (3f - age * 2.5f).coerceAtLeast(0.5f)
            val pColor = when {
                age < 0.33f -> particleA
                age < 0.66f -> particleB
                else -> particleC
            }
            drawCircle(
                color = pColor.copy(alpha = pAlpha),
                radius = pRadius,
                center = Offset(cx + cos(pAngle) * pDist, cy + sin(pAngle) * pDist),
            )
        }
    }
}
