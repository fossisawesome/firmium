package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.Dp
import androidx.wear.compose.material.Icon
import coil.compose.AsyncImage
import com.fossisawesome.firmium.data.api.WatchAuthManager

// Cover art thumbnail with a music-note placeholder behind it. Small, single-purpose —
// unlike the phone's CoverImage.kt, no loading spinner (thumbnails are small enough that
// a spinner would be more distracting than useful on a watch).
@Composable
fun WatchCoverImage(url: String?, contentDescription: String?, size: Dp, modifier: Modifier = Modifier) {
    Box(modifier.size(size).background(Color.DarkGray), contentAlignment = Alignment.Center) {
        if (!url.isNullOrBlank()) {
            AsyncImage(
                model = url,
                contentDescription = contentDescription,
                modifier = Modifier.fillMaxSize(),
                contentScale = ContentScale.Crop,
            )
        } else {
            Icon(imageVector = Icons.Filled.MusicNote, contentDescription = null)
        }
    }
}

// Null-safe cover URL lookup — every list screen has a nullable coverArt field.
fun WatchAuthManager.safeCoverArtUrl(coverArt: String?, size: Int): String? =
    coverArt?.let { coverArtUrl(it, size) }
