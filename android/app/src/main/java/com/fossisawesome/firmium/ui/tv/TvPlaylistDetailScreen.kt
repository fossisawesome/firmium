package com.fossisawesome.firmium.ui.tv

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

@Composable
fun TvPlaylistDetailScreen(
    title: String,
    tracks: List<Song>,
    onLoad: () -> Unit,
    onPlayAll: (List<Song>, Int) -> Unit,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val firstFocusRequester = remember { FocusRequester() }

    LaunchedEffect(Unit) { onLoad() }
    LaunchedEffect(tracks) {
        if (tracks.isNotEmpty()) firstFocusRequester.requestFocus()
    }
    BackHandler { onBack() }

    Column(modifier = Modifier.fillMaxSize().padding(48.dp)) {
        Text(text = title, color = colors.text, fontSize = 24.sp, modifier = Modifier.padding(bottom = 8.dp))
        if (tracks.isNotEmpty()) {
            TvTile(onClick = { onPlayAll(tracks, 0) }, colors = colors, modifier = Modifier.padding(bottom = 16.dp)) {
                Text(text = "Play All", color = colors.text, fontSize = 14.sp, modifier = Modifier.padding(horizontal = 20.dp, vertical = 12.dp))
            }
        }
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            items(tracks.size) { index ->
                val track = tracks[index]
                TvTile(
                    onClick = { onPlayAll(tracks, index) },
                    colors = colors,
                    modifier = Modifier
                        .fillMaxWidth()
                        .then(if (index == 0) Modifier.focusRequester(firstFocusRequester) else Modifier),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(text = track.title, color = colors.text, fontSize = 14.sp, maxLines = 1, modifier = Modifier.weight(1f))
                        Text(text = track.artist, color = colors.muted, fontSize = 12.sp, maxLines = 1)
                    }
                }
            }
        }
    }
}
