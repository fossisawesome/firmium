package com.fossisawesome.firmium.audio

import androidx.media3.common.C
import androidx.media3.common.audio.AudioProcessor
import androidx.media3.common.audio.BaseAudioProcessor
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.sin

private const val BLOCK_SIZE = 1024 // power of two, FFT input size

// Media3 AudioProcessor that taps decoded PCM directly in ExoPlayer's audio pipeline and computes
// waveform + FFT for the visualizer. Replaces android.media.audiofx.Visualizer, which requires
// RECORD_AUDIO and behaved inconsistently across OEMs (silent no-ops, capture races against
// session id assignment). This processor sees the exact samples ExoPlayer is about to play,
// in-process — no permission needed — and is a pure passthrough: it copies input to output
// unchanged, only reading the samples as a side effect. `isActive` (from BaseAudioProcessor)
// reports false whenever `onConfigure` can't handle the format, so the pipeline bypasses it
// entirely rather than breaking playback.
class VisualizerAudioProcessor : BaseAudioProcessor() {

    /** Invoked off the audio thread whenever a new analysis block is ready. */
    var onData: ((waveform: FloatArray, magnitudes: FloatArray, bass: Float) -> Unit)? = null

    private var channelCount = 0
    private var encoding = C.ENCODING_INVALID
    private val monoBuffer = FloatArray(BLOCK_SIZE)
    private var monoFill = 0
    private val fftRe = FloatArray(BLOCK_SIZE)
    private val fftIm = FloatArray(BLOCK_SIZE)
    private val window = FloatArray(BLOCK_SIZE) { i ->
        (0.5f - 0.5f * cos(2f * PI.toFloat() * i / (BLOCK_SIZE - 1)))
    }
    private var bass = 0f

    override fun onConfigure(inputAudioFormat: AudioProcessor.AudioFormat): AudioProcessor.AudioFormat {
        if (inputAudioFormat.channelCount <= 0 ||
            (inputAudioFormat.encoding != C.ENCODING_PCM_16BIT && inputAudioFormat.encoding != C.ENCODING_PCM_FLOAT)
        ) {
            throw AudioProcessor.UnhandledAudioFormatException(inputAudioFormat)
        }
        channelCount = inputAudioFormat.channelCount
        encoding = inputAudioFormat.encoding
        return inputAudioFormat
    }

    override fun queueInput(inputBuffer: ByteBuffer) {
        val remaining = inputBuffer.remaining()
        if (remaining == 0) return
        val outputBuffer = replaceOutputBuffer(remaining)

        inputBuffer.order(ByteOrder.LITTLE_ENDIAN)
        if (encoding == C.ENCODING_PCM_16BIT) {
            val shorts = inputBuffer.asShortBuffer()
            val frameCount = shorts.remaining() / channelCount
            for (f in 0 until frameCount) {
                var sum = 0
                for (c in 0 until channelCount) sum += shorts.get()
                pushSample((sum / channelCount) / 32768f)
            }
        } else {
            val floats = inputBuffer.asFloatBuffer()
            val frameCount = floats.remaining() / channelCount
            for (f in 0 until frameCount) {
                var sum = 0f
                for (c in 0 until channelCount) sum += floats.get()
                pushSample(sum / channelCount)
            }
        }

        outputBuffer.put(inputBuffer)
        outputBuffer.flip()
        inputBuffer.position(inputBuffer.limit())
    }

    private fun pushSample(sample: Float) {
        monoBuffer[monoFill++] = sample
        if (monoFill == BLOCK_SIZE) {
            analyzeBlock()
            monoFill = 0
        }
    }

    private fun analyzeBlock() {
        for (i in 0 until BLOCK_SIZE) {
            fftRe[i] = monoBuffer[i] * window[i]
            fftIm[i] = 0f
        }
        fft(fftRe, fftIm)

        val bins = BLOCK_SIZE / 2
        val mags = FloatArray(bins)
        var bassSum = 0f
        var bassCount = 0
        val bassCutoff = max(1, bins / 8)
        for (k in 0 until bins) {
            val mag = (hypot(fftRe[k], fftIm[k]) / (BLOCK_SIZE / 4f)).coerceIn(0f, 1f)
            mags[k] = mag
            if (k in 1..bassCutoff) { bassSum += mag; bassCount++ }
        }
        val target = if (bassCount > 0) (bassSum / bassCount * 1.8f).coerceIn(0f, 1f) else 0f
        // Fast attack, slow decay — reads as "on the beat" rather than mushy.
        bass = if (target > bass) target else bass * 0.86f + target * 0.14f

        onData?.invoke(monoBuffer.copyOf(), mags, bass)
    }

    override fun onFlush() {
        monoFill = 0
        bass = 0f
    }

    override fun onReset() {
        monoFill = 0
        bass = 0f
        channelCount = 0
        encoding = C.ENCODING_INVALID
    }
}

// In-place iterative radix-2 Cooley-Tukey FFT. `re`/`im` length must be a power of two.
private fun fft(re: FloatArray, im: FloatArray) {
    val n = re.size
    var j = 0
    for (i in 1 until n) {
        var bit = n shr 1
        while (j and bit != 0) { j = j xor bit; bit = bit shr 1 }
        j = j or bit
        if (i < j) {
            var t = re[i]; re[i] = re[j]; re[j] = t
            t = im[i]; im[i] = im[j]; im[j] = t
        }
    }
    var len = 2
    while (len <= n) {
        val ang = -2f * PI.toFloat() / len
        val wr = cos(ang)
        val wi = sin(ang)
        var i = 0
        while (i < n) {
            var curWr = 1f
            var curWi = 0f
            for (k in 0 until len / 2) {
                val uRe = re[i + k]; val uIm = im[i + k]
                val idx2 = i + k + len / 2
                val vRe = re[idx2] * curWr - im[idx2] * curWi
                val vIm = re[idx2] * curWi + im[idx2] * curWr
                re[i + k] = uRe + vRe; im[i + k] = uIm + vIm
                re[idx2] = uRe - vRe; im[idx2] = uIm - vIm
                val nextWr = curWr * wr - curWi * wi
                val nextWi = curWr * wi + curWi * wr
                curWr = nextWr; curWi = nextWi
            }
            i += len
        }
        len = len shl 1
    }
}
