package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.components.CoverImage
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.AlbumListState

@Composable
fun TvAlbumListScreen(
    state: AlbumListState,
    coverUrlFor: (String?) -> String?,
    onLoad: () -> Unit,
    onAlbumClick: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val firstFocusRequester = remember { FocusRequester() }

    LaunchedEffect(Unit) { onLoad() }
    LaunchedEffect(state.albums) {
        if (state.albums.isNotEmpty()) firstFocusRequester.requestFocus()
    }

    LazyVerticalGrid(
        columns = GridCells.Fixed(6),
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(48.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        items(state.albums.size) { index ->
            val album = state.albums[index]
            TvTile(
                onClick = { onAlbumClick(album.id) },
                colors = colors,
                modifier = Modifier
                    .size(width = 160.dp, height = 200.dp)
                    .then(if (index == 0) Modifier.focusRequester(firstFocusRequester) else Modifier),
            ) {
                Column {
                    CoverImage(url = coverUrlFor(album.coverArt), contentDescription = album.name, size = 160.dp)
                    Text(text = album.name, color = colors.text, fontSize = 13.sp, maxLines = 1, modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp))
                    Text(text = album.artist, color = colors.muted, fontSize = 11.sp, maxLines = 1, modifier = Modifier.padding(horizontal = 4.dp))
                }
            }
        }
    }
}
