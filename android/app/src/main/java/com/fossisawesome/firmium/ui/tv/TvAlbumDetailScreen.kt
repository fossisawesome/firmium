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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
import com.fossisawesome.firmium.viewmodel.AlbumDetailState

@Composable
fun TvAlbumDetailScreen(
    albumId: String,
    state: AlbumDetailState,
    coverUrlFor: (String?) -> String?,
    onLoad: (String) -> Unit,
    onPlayAt: (List<Song>, Int) -> Unit,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val firstFocusRequester = remember { FocusRequester() }

    LaunchedEffect(albumId) { onLoad(albumId) }
    LaunchedEffect(state.album) {
        if (state.album != null) firstFocusRequester.requestFocus()
    }
    BackHandler { onBack() }

    val album = state.album ?: return

    Row(modifier = Modifier.fillMaxSize().padding(48.dp)) {
        Column(modifier = Modifier.padding(end = 32.dp)) {
            CoverImage(url = coverUrlFor(album.coverArt), contentDescription = album.name, size = 240.dp)
            Text(text = album.name, color = colors.text, fontSize = 20.sp, modifier = Modifier.padding(top = 16.dp).fillMaxWidth())
            Text(text = album.artist, color = colors.muted, fontSize = 14.sp)
            album.year?.let { Text(text = it.toString(), color = colors.muted, fontSize = 13.sp) }
        }
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            items(album.tracks.size) { index ->
                val track = album.tracks[index]
                TvTile(
                    onClick = { onPlayAt(album.tracks, index) },
                    colors = colors,
                    modifier = Modifier
                        .fillMaxWidth()
                        .then(if (index == 0) Modifier.focusRequester(firstFocusRequester) else Modifier),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "${track.track ?: index + 1}",
                            color = colors.muted,
                            fontSize = 13.sp,
                            modifier = Modifier.padding(end = 16.dp),
                        )
                        Text(
                            text = track.title,
                            color = colors.text,
                            fontSize = 14.sp,
                            maxLines = 1,
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
            }
        }
    }
}
