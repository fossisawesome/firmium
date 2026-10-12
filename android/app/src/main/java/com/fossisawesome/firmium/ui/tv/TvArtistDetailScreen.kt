package com.fossisawesome.firmium.ui.tv

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.CoverImage
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.ArtistDetailState

@Composable
fun TvArtistDetailScreen(
    artistId: String,
    state: ArtistDetailState,
    coverUrlFor: (String?) -> String?,
    onLoad: (String) -> Unit,
    onAlbumClick: (String) -> Unit,
    onPlayAll: (List<Song>, Int) -> Unit,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val firstFocusRequester = remember { FocusRequester() }

    LaunchedEffect(artistId) { onLoad(artistId) }
    LaunchedEffect(state.detail) {
        if (state.detail != null) firstFocusRequester.requestFocus()
    }
    BackHandler { onBack() }

    val detail = state.detail ?: return

    Column(modifier = Modifier.fillMaxSize().padding(top = 48.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 48.dp, end = 48.dp, bottom = 24.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = detail.artist.name,
                color = colors.text,
                fontSize = 24.sp,
                modifier = Modifier.weight(1f),
            )
            TvActionButton(
                onClick = { onPlayAll(state.allSongs, 0) },
                colors = colors,
                modifier = Modifier.focusRequester(firstFocusRequester),
            ) {
                Text(text = "Play All", color = colors.text, fontSize = 13.sp)
            }
        }
        LazyVerticalGrid(
            columns = GridCells.Fixed(6),
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(horizontal = 48.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            items(detail.albums.size) { index ->
                val album = detail.albums[index]
                TvTile(
                    onClick = { onAlbumClick(album.id) },
                    colors = colors,
                    modifier = Modifier.size(width = 160.dp, height = 200.dp),
                ) {
                    Column {
                        CoverImage(url = coverUrlFor(album.coverArt), contentDescription = album.name, size = 160.dp)
                        Text(text = album.name, color = colors.text, fontSize = 13.sp, maxLines = 1, modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp))
                    }
                }
            }
        }
    }
}
