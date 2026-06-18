package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

// Spotify-style playlist cover built from the first distinct song covers:
// 1 cover fills the square, 2 split side-by-side, 3 = one tall left + two stacked
// right, 4 = 2x2 grid. Falls back to CoverImage's placeholder when empty.
@Composable
fun PlaylistMosaic(
    coverUrls: List<String?>,
    modifier: Modifier = Modifier,
) {
    val tiles = coverUrls.filterNotNull().distinct().take(4)
    when (tiles.size) {
        0, 1 -> CoverImage(url = tiles.firstOrNull(), contentDescription = null, modifier = modifier)
        2 -> Row(modifier) {
            CoverImage(tiles[0], null, Modifier.weight(1f).fillMaxHeight())
            CoverImage(tiles[1], null, Modifier.weight(1f).fillMaxHeight())
        }
        3 -> Row(modifier) {
            CoverImage(tiles[0], null, Modifier.weight(1f).fillMaxHeight())
            Column(Modifier.weight(1f).fillMaxHeight()) {
                CoverImage(tiles[1], null, Modifier.weight(1f).fillMaxWidth())
                CoverImage(tiles[2], null, Modifier.weight(1f).fillMaxWidth())
            }
        }
        else -> Column(modifier) {
            Row(Modifier.weight(1f).fillMaxWidth()) {
                CoverImage(tiles[0], null, Modifier.weight(1f).fillMaxHeight())
                CoverImage(tiles[1], null, Modifier.weight(1f).fillMaxHeight())
            }
            Row(Modifier.weight(1f).fillMaxWidth()) {
                CoverImage(tiles[2], null, Modifier.weight(1f).fillMaxHeight())
                CoverImage(tiles[3], null, Modifier.weight(1f).fillMaxHeight())
            }
        }
    }
}
