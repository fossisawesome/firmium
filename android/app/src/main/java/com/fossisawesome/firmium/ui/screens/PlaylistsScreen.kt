package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Cloud
import androidx.compose.material.icons.filled.Delete
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.ui.theme.LocalUiTheme
import com.fossisawesome.firmium.viewmodel.PlaylistListItem
import com.fossisawesome.firmium.viewmodel.PlaylistsUiState

@Composable
fun PlaylistsScreen(
    state: PlaylistsUiState,
    coverUrlFor: (String?) -> String?,
    onPlaylistClick: (String) -> Unit,
    onCreate: (String) -> Unit,
    onDelete: (String) -> Unit,
    onSync: (String) -> Unit,
    onRefreshServer: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val spotify = LocalUiTheme.current == "spotify"
    var showCreateDialog by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) { onRefreshServer() }

    Column(modifier = Modifier.fillMaxSize()) {
        // .pl-list-header: section-header "PLAYLISTS" left + "+ New" button right
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 10.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (spotify) {
                Text(
                    "Playlists",
                    fontSize = 20.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
                    color = colors.text,
                    modifier = Modifier.weight(1f),
                )
            } else {
                Text(
                    "Playlists".uppercase(),
                    fontSize = 12.sp, fontFamily = LocalAppFontFamily.current,
                    color = colors.muted, letterSpacing = 1.sp,
                    modifier = Modifier.weight(1f),
                )
            }
            // .pl-new-btn: surface2 bg, 1dp border, accent text, monospace 11sp, 3dp radius
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(3.dp))
                    .background(colors.surface2)
                    .border(1.dp, colors.accent.copy(alpha = 0.5f), RoundedCornerShape(3.dp))
                    .clickable { showCreateDialog = true }
                    .padding(horizontal = 14.dp, vertical = 6.dp),
            ) {
                Text("+ New", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.accent)
            }
        }

        if (state.items.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No playlists yet", fontFamily = LocalAppFontFamily.current, fontSize = 14.sp, color = colors.muted)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                items(state.items, key = { it.id }) { item ->
                    PlaylistRow(
                        item = item,
                        coverUrlFor = coverUrlFor,
                        onClick = { onPlaylistClick(item.id) },
                        onDelete = if (item is PlaylistListItem.Local) ({ onDelete(item.id) }) else null,
                        onSync = if (item is PlaylistListItem.Local && !item.isSynced) ({ onSync(item.id) }) else null,
                    )
                    FirmiumDivider()
                }
            }
        }
    }

    if (showCreateDialog) {
        FirmiumCreatePlaylistDialog(
            onConfirm = { name ->
                if (name.isNotBlank()) onCreate(name)
                showCreateDialog = false
            },
            onDismiss = { showCreateDialog = false },
        )
    }
}

// .pl-card: padding 12/10dp, gap 16dp. Art: 48×48dp, surface2, 8dp radius.
@Composable
private fun PlaylistRow(
    item: PlaylistListItem,
    coverUrlFor: (String?) -> String?,
    onClick: () -> Unit,
    onDelete: (() -> Unit)?,
    onSync: (() -> Unit)?,
) {
    val colors = LocalFirmiumColors.current
    val spotify = LocalUiTheme.current == "spotify"
    val artSize = if (spotify) 56.dp else 48.dp
    // Local/synced playlists have their tracks in memory, so build a true mosaic from
    // the first distinct song covers. Server-only playlists only carry a single server
    // cover id (no track list loaded yet), so fall back to that — avoids per-row fetches.
    val coverUrls = when (item) {
        is PlaylistListItem.Local -> item.playlist.tracks.map { coverUrlFor(it.coverArt) }
        is PlaylistListItem.ServerOnly -> listOf(coverUrlFor(item.server.coverArt))
    }
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }
            .padding(horizontal = 10.dp, vertical = if (spotify) 14.dp else 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // .pl-card-art: 48×48dp, surface2 bg, 8dp radius
        Box(
            modifier = Modifier.size(artSize).clip(RoundedCornerShape(if (spotify) 10.dp else 8.dp)).background(colors.surface2),
            contentAlignment = Alignment.Center,
        ) {
            PlaylistMosaic(coverUrls = coverUrls, modifier = Modifier.fillMaxSize())
        }
        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(
                    item.name, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
                    fontSize = if (spotify) 15.sp else 14.sp, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
                if (item.isSynced) {
                    FirmiumIcon(Icons.Default.Cloud, contentDescription = "Synced to server",
                        tint = colors.muted, modifier = Modifier.size(14.dp))
                }
            }
            Text(
                "${item.trackCount} track${if (item.trackCount != 1) "s" else ""}",
                fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted,
            )
        }
        // Sync — small icon button for local-only playlists not yet on the server
        if (onSync != null) {
            FirmiumIconButton(onClick = onSync, modifier = Modifier.size(36.dp)) {
                FirmiumIcon(Icons.Default.Cloud, contentDescription = "Sync to server",
                    tint = colors.accent, modifier = Modifier.size(18.dp))
            }
        }
        // Delete — small icon button (only for local playlists)
        if (onDelete != null) {
            FirmiumIconButton(onClick = onDelete, modifier = Modifier.size(36.dp)) {
                FirmiumIcon(Icons.Default.Delete, contentDescription = "Delete",
                    tint = colors.error, modifier = Modifier.size(18.dp))
            }
        }
    }
}

// Custom dialog matching the old .dialog CSS: surface bg, border, monospace font
@Composable
private fun FirmiumCreatePlaylistDialog(
    onConfirm: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var name by remember { mutableStateOf("") }

    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = androidx.compose.ui.Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(8.dp))
                .background(colors.surface)
                .border(1.dp, colors.border, RoundedCornerShape(8.dp))
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                "New Playlist", fontSize = 16.sp, fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current, color = colors.text,
            )
            FirmiumTextField(
                value = name,
                onValueChange = { name = it },
                placeholder = "Playlist name",
                modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
            )
            Row(
                modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End),
            ) {
                Text(
                    "Cancel", fontSize = 13.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted,
                    modifier = androidx.compose.ui.Modifier.clickable { onDismiss() }.padding(8.dp),
                )
                Text(
                    "Create", fontSize = 13.sp, fontFamily = LocalAppFontFamily.current,
                    color = if (name.isNotBlank()) colors.accent else colors.muted,
                    modifier = androidx.compose.ui.Modifier
                        .clickable(enabled = name.isNotBlank()) { onConfirm(name) }
                        .padding(8.dp),
                )
            }
        }
    }
}
