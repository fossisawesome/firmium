package com.fossisawesome.firmium.ui.components

import android.graphics.drawable.BitmapDrawable
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.palette.graphics.Palette
import coil.imageLoader
import coil.request.ImageRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

// Three vibrant colors pulled from cover art for the orb visualizer.
data class OrbPalette(
    val primary: Color,    // vibrant swatch — brightest pop color
    val secondary: Color,  // light vibrant — highlights / ring ticks
    val tertiary: Color,   // muted — particles
)

private val DefaultOrbPalette = OrbPalette(
    primary = Color(0xFF7C5CFF),
    secondary = Color(0xFFAA88FF),
    tertiary = Color(0xFF5533CC),
)

@Composable
fun rememberOrbPalette(coverUrl: String?): OrbPalette {
    val context = LocalContext.current
    var palette by remember { mutableStateOf(DefaultOrbPalette) }
    LaunchedEffect(coverUrl) {
        if (coverUrl == null) { palette = DefaultOrbPalette; return@LaunchedEffect }
        withContext(Dispatchers.IO) {
            try {
                val req = ImageRequest.Builder(context).data(coverUrl).allowHardware(false).build()
                val bmp = (context.imageLoader.execute(req).drawable as? BitmapDrawable)?.bitmap
                    ?: return@withContext
                val p = Palette.from(bmp).generate()
                val primary = p.vibrantSwatch ?: p.dominantSwatch ?: return@withContext
                val secondary = p.lightVibrantSwatch ?: p.lightMutedSwatch ?: primary
                val tertiary = p.mutedSwatch ?: p.darkVibrantSwatch ?: primary
                palette = OrbPalette(
                    primary = Color(primary.rgb),
                    secondary = Color(secondary.rgb),
                    tertiary = Color(tertiary.rgb),
                )
            } catch (_: Exception) {}
        }
    }
    return palette
}

// Extracts the dominant color from cover art, darkened to 22% (same formula as the
// Svelte version's full-screen player background). Returns null while loading or
// if extraction fails (e.g. no cover art).
@Composable
fun rememberDominantColor(coverUrl: String?): Color? {
    val context = LocalContext.current
    var color by remember { mutableStateOf<Color?>(null) }
    LaunchedEffect(coverUrl) {
        if (coverUrl == null) { color = null; return@LaunchedEffect }
        withContext(Dispatchers.IO) {
            try {
                val req = ImageRequest.Builder(context).data(coverUrl).allowHardware(false).build()
                val bmp = (context.imageLoader.execute(req).drawable as? BitmapDrawable)?.bitmap
                    ?: return@withContext
                val palette = Palette.from(bmp).generate()
                val swatch = palette.dominantSwatch ?: palette.vibrantSwatch ?: palette.mutedSwatch
                swatch?.let {
                    val r = ((it.rgb shr 16 and 0xFF) * 0.22f).toInt()
                    val g = ((it.rgb shr 8 and 0xFF) * 0.22f).toInt()
                    val b = ((it.rgb and 0xFF) * 0.22f).toInt()
                    color = Color(r, g, b)
                }
            } catch (_: Exception) { color = null }
        }
    }
    return color
}
