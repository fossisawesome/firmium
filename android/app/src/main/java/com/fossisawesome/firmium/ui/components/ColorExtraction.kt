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
