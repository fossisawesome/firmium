package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
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
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.PlaylistsUiState

@Composable
fun PlaylistsScreen(
    state: PlaylistsUiState,
    onPlaylistClick: (String) -> Unit,
    onCreate: (String) -> Unit,
    onDelete: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var showCreateDialog by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxSize()) {
        // .pl-list-header: section-header "PLAYLISTS" left + "+ New" button right
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 10.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "Playlists".uppercase(),
                fontSize = 12.sp, fontFamily = FontFamily.Monospace,
                color = colors.muted, letterSpacing = 1.sp,
                modifier = Modifier.weight(1f),
            )
            // .pl-new-btn: surface2 bg, 1dp border, accent text, monospace 11sp, 3dp radius
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(3.dp))
                    .background(colors.surface2)
                    .border(1.dp, colors.accent.copy(alpha = 0.5f), RoundedCornerShape(3.dp))
                    .clickable { showCreateDialog = true }
                    .padding(horizontal = 14.dp, vertical = 6.dp),
            ) {
                Text("+ New", fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.accent)
            }
        }

        if (state.playlists.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No playlists yet", fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = colors.muted)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                items(state.playlists, key = { it.id }) { playlist ->
                    PlaylistRow(
                        playlist = playlist,
                        onClick = { onPlaylistClick(playlist.id) },
                        onDelete = { onDelete(playlist.id) },
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
private fun PlaylistRow(playlist: Playlist, onClick: () -> Unit, onDelete: () -> Unit) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }
            .padding(horizontal = 10.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // .pl-card-art: 48×48dp, surface2 bg, 8dp radius
        Box(
            modifier = Modifier.size(48.dp).clip(RoundedCornerShape(8.dp)).background(colors.surface2),
            contentAlignment = Alignment.Center,
        ) {
            Text("♪", fontSize = 20.sp, color = colors.muted)
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(
                playlist.name, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace,
                fontSize = 14.sp, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            Text(
                "${playlist.tracks.size} track${if (playlist.tracks.size != 1) "s" else ""}",
                fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
            )
        }
        // Delete — small icon button
        FirmiumIconButton(onClick = onDelete, modifier = Modifier.size(36.dp)) {
            FirmiumIcon(Icons.Default.Delete, contentDescription = "Delete",
                tint = colors.error, modifier = Modifier.size(18.dp))
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
                fontFamily = FontFamily.Monospace, color = colors.text,
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
                    "Cancel", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
                    modifier = androidx.compose.ui.Modifier.clickable { onDismiss() }.padding(8.dp),
                )
                Text(
                    "Create", fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                    color = if (name.isNotBlank()) colors.accent else colors.muted,
                    modifier = androidx.compose.ui.Modifier
                        .clickable(enabled = name.isNotBlank()) { onConfirm(name) }
                        .padding(8.dp),
                )
            }
        }
    }
}
