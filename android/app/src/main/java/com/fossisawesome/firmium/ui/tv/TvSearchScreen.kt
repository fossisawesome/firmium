package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.CoverImage
import com.fossisawesome.firmium.ui.components.FirmiumTextField
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.SearchState

@Composable
fun TvSearchScreen(
    state: SearchState,
    coverUrlFor: (String?) -> String?,
    onQueryChange: (String) -> Unit,
    onPlaySong: (List<Song>, Int) -> Unit,
    onAlbumClick: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current

    Column(modifier = Modifier.fillMaxSize().padding(48.dp)) {
        FirmiumTextField(
            value = state.query,
            onValueChange = onQueryChange,
            placeholder = "Search albums and songs",
            modifier = Modifier.width(480.dp).padding(bottom = 24.dp),
        )

        if (state.albums.isNotEmpty()) {
            Text(text = "Albums", color = colors.text, fontSize = 16.sp, modifier = Modifier.padding(bottom = 8.dp))
            LazyRow(
                horizontalArrangement = Arrangement.spacedBy(16.dp),
                contentPadding = PaddingValues(bottom = 24.dp),
            ) {
                items(state.albums.size) { index ->
                    val album = state.albums[index]
                    TvTile(onClick = { onAlbumClick(album.id) }, colors = colors, modifier = Modifier.size(width = 140.dp, height = 180.dp)) {
                        Column {
                            CoverImage(url = coverUrlFor(album.coverArt), contentDescription = album.name, size = 140.dp)
                            Text(text = album.name, color = colors.text, fontSize = 12.sp, maxLines = 1, modifier = Modifier.padding(4.dp))
                        }
                    }
                }
            }
        }

        if (state.songs.isNotEmpty()) {
            Text(text = "Songs", color = colors.text, fontSize = 16.sp, modifier = Modifier.padding(bottom = 8.dp))
            LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                items(state.songs.size) { index ->
                    val song = state.songs[index]
                    TvTile(onClick = { onPlaySong(state.songs, index) }, colors = colors, modifier = Modifier.fillMaxWidth()) {
                        Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(text = song.title, color = colors.text, fontSize = 14.sp, maxLines = 1, modifier = Modifier.weight(1f))
                            Text(text = song.artist, color = colors.muted, fontSize = 12.sp, maxLines = 1)
                        }
                    }
                }
            }
        }
    }
}
