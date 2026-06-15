package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.compose.AsyncImagePainter
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Consistent cover art image with a music note placeholder behind it.
// Uses AsyncImage (not SubcomposeAsyncImage) to avoid reloading images on recomposition
// when panels open — AsyncImage doesn't use sub-composition so it doesn't re-trigger
// on state changes in ancestor composables.
@Composable
fun CoverImage(
    url: String?,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    size: Dp? = null,
) {
    val mod = if (size != null) modifier.size(size) else modifier

    var isLoading by remember(url) { mutableStateOf(!url.isNullOrBlank()) }

    Box(mod) {
        // Music-note placeholder always rendered underneath; AsyncImage covers it once loaded.
        PlaceholderCover(Modifier.fillMaxSize())

        if (!url.isNullOrBlank()) {
            AsyncImage(
                model = url,
                contentDescription = contentDescription,
                modifier = Modifier.fillMaxSize(),
                contentScale = ContentScale.Crop,
                onState = { state -> isLoading = state is AsyncImagePainter.State.Loading },
            )
        }

        if (isLoading) {
            FirmiumSpinner(
                color = LocalFirmiumColors.current.muted,
                modifier = Modifier.size(20.dp).align(Alignment.Center),
            )
        }
    }
}

@Composable
private fun PlaceholderCover(modifier: Modifier) {
    val colors = LocalFirmiumColors.current
    Box(
        modifier = modifier.background(colors.surface2),
        contentAlignment = Alignment.Center,
    ) {
        FirmiumIcon(
            imageVector = Icons.Default.MusicNote,
            // Decorative — CoverImage's contentDescription param describes the actual artwork.
            contentDescription = null,
            tint = colors.muted,
            modifier = Modifier.size(32.dp),
        )
    }
}
