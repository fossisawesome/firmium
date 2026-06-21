package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.DownloadDone
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

private enum class DownloadState { Idle, Loading, Downloaded, Error }

// Download icon button shared by AlbumRow, AlbumTrackRow, and TrackRow. Shows a spinner while
// downloading; on success it settles on a persistent "downloaded" check (so a fresh download is
// visibly marked even in server mode) instead of reverting to the download icon. Errors are
// transient. `initiallyDownloaded` seeds the persistent state from the local-library scan.
// Still clickable when downloaded so server-mode users can re-download.
@Composable
fun DownloadButton(
    onDownload: suspend () -> Result<Unit>,
    modifier: Modifier = Modifier,
    buttonSize: Dp = 36.dp,
    iconSize: Dp = 18.dp,
    initiallyDownloaded: Boolean = false,
) {
    val colors = LocalFirmiumColors.current
    var state by remember(initiallyDownloaded) {
        mutableStateOf(if (initiallyDownloaded) DownloadState.Downloaded else DownloadState.Idle)
    }
    val scope = rememberCoroutineScope()

    FirmiumIconButton(
        onClick = {
            if (state == DownloadState.Loading) return@FirmiumIconButton
            scope.launch {
                state = DownloadState.Loading
                val result = onDownload()
                if (result.isSuccess) {
                    state = DownloadState.Downloaded
                } else {
                    state = DownloadState.Error
                    delay(2000)
                    state = DownloadState.Idle
                }
            }
        },
        modifier = modifier.size(buttonSize),
    ) {
        when (state) {
            DownloadState.Loading -> FirmiumSpinner(color = colors.muted, modifier = Modifier.size(iconSize))
            DownloadState.Downloaded -> FirmiumIcon(Icons.Default.DownloadDone, contentDescription = "Downloaded", tint = colors.accent, modifier = Modifier.size(iconSize))
            DownloadState.Error -> FirmiumIcon(Icons.Default.ErrorOutline, contentDescription = "Download failed", tint = colors.error, modifier = Modifier.size(iconSize))
            DownloadState.Idle -> FirmiumIcon(Icons.Default.Download, contentDescription = "Download", tint = colors.muted, modifier = Modifier.size(iconSize))
        }
    }
}
