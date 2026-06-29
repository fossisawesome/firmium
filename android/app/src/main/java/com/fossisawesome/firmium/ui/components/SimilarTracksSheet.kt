package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.SimilarTracksState
import kotlin.math.roundToInt

// Similar Tracks bottom sheet, powered by the sonicSimilarity OpenSubsonic extension.
// Tapping a row plays the full similar-tracks list starting at that track.
@Composable
fun SimilarTracksSheet(
    state: SimilarTracksState,
    onDismiss: () -> Unit,
    onPlayAt: (songs: List<Song>, index: Int) -> Unit,
) {
    val colors = LocalFirmiumColors.current

    FirmiumBottomSheet(onDismiss = onDismiss) {
        Text(
            text = "Similar Tracks",
            fontSize = 16.sp,
            fontFamily = LocalAppFontFamily.current,
            color = colors.text,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )

        when {
            state.isLoading -> Box(
                modifier = Modifier.fillMaxWidth().height(120.dp),
                contentAlignment = Alignment.Center,
            ) {
                FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
            }
            state.error != null || state.matches.isEmpty() -> Box(
                modifier = Modifier.fillMaxWidth().height(120.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = state.error ?: "No similar tracks found",
                    fontSize = 14.sp,
                    fontFamily = LocalAppFontFamily.current,
                    fontStyle = FontStyle.Italic,
                    color = colors.muted,
                )
            }
            else -> LazyColumn(
                modifier = Modifier.fillMaxWidth().heightIn(max = 480.dp),
                contentPadding = PaddingValues(bottom = 32.dp),
            ) {
                items(state.matches) { match ->
                    SimilarTrackItem(
                        match = match,
                        onClick = { onPlayAt(state.matches.map { it.song }, state.matches.indexOf(match)) },
                    )
                    FirmiumDivider(color = colors.border)
                }
            }
        }
    }
}

@Composable
private fun SimilarTrackItem(
    match: ApiClient.SimilarMatch,
    onClick: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val song = match.song
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = song.title,
                fontSize = 14.sp,
                fontFamily = LocalAppFontFamily.current,
                color = colors.text,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = song.displayArtist ?: song.artist,
                fontSize = 12.sp,
                fontFamily = LocalAppFontFamily.current,
                color = colors.muted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }

        Spacer(Modifier.width(12.dp))

        Text(
            text = "${(match.similarity * 100).roundToInt()}%",
            fontSize = 12.sp,
            fontFamily = LocalAppFontFamily.current,
            color = colors.muted,
        )
    }
}
