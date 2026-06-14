package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private enum class DownloadState { Idle, Loading, Done, Error }

// Download icon button shared by AlbumRow, AlbumTrackRow, and TrackRow. Shows a spinner while
// downloading and a check/error icon briefly afterwards, mirroring the desktop's transient
// status pill (TrackRow.svelte / AlbumRow.svelte).
@Composable
fun DownloadButton(
    onDownload: suspend () -> Result<Unit>,
    modifier: Modifier = Modifier,
    buttonSize: Dp = 36.dp,
    iconSize: Dp = 18.dp,
) {
    val colors = LocalFirmiumColors.current
    var state by remember { mutableStateOf(DownloadState.Idle) }
    val scope = rememberCoroutineScope()

    FirmiumIconButton(
        onClick = {
            if (state == DownloadState.Loading) return@FirmiumIconButton
            scope.launch {
                state = DownloadState.Loading
                val result = onDownload()
                state = if (result.isSuccess) DownloadState.Done else DownloadState.Error
                delay(2000)
                state = DownloadState.Idle
            }
        },
        modifier = modifier.size(buttonSize),
    ) {
        when (state) {
            DownloadState.Loading -> FirmiumSpinner(color = colors.muted, modifier = Modifier.size(iconSize))
            DownloadState.Done -> FirmiumIcon(Icons.Default.Check, contentDescription = "Downloaded", tint = colors.accent, modifier = Modifier.size(iconSize))
            DownloadState.Error -> FirmiumIcon(Icons.Default.ErrorOutline, contentDescription = "Download failed", tint = colors.error, modifier = Modifier.size(iconSize))
            DownloadState.Idle -> FirmiumIcon(Icons.Default.Download, contentDescription = "Download", tint = colors.muted, modifier = Modifier.size(iconSize))
        }
    }
}
