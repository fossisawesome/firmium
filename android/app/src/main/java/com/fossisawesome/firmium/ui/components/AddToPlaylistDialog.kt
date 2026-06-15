package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.automirrored.filled.PlaylistPlay
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Sheet for picking an existing playlist to add tracks to, or creating a new one.
@Composable
fun AddToPlaylistDialog(
    playlists: List<Playlist>,
    onAddTo: (playlistId: String) -> Unit,
    onCreateAndAdd: (name: String) -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var showCreate by remember { mutableStateOf(false) }
    var newName by remember { mutableStateOf("") }

    FirmiumBottomSheet(onDismiss = onDismiss) {
        Text(
            "Add to playlist",
            fontSize = 16.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Bold,
            color = colors.text,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )
        FirmiumDivider()

        if (showCreate) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                FirmiumTextField(
                    value = newName,
                    onValueChange = { newName = it },
                    label = "Playlist name",
                    modifier = Modifier.fillMaxWidth(),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    // Cancel button
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .clip(RoundedCornerShape(2.dp))
                            .border(1.dp, colors.border, RoundedCornerShape(2.dp))
                            .clickable { showCreate = false; newName = "" }
                            .padding(vertical = 10.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text("Cancel", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                    }
                    // Create & Add button
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .clip(RoundedCornerShape(2.dp))
                            .background(if (newName.isNotBlank()) colors.accent else colors.surface2)
                            .alpha(if (newName.isNotBlank()) 1f else 0.5f)
                            .clickable(enabled = newName.isNotBlank()) { onCreateAndAdd(newName); onDismiss() }
                            .padding(vertical = 10.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text("Create & Add", fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                            color = if (newName.isNotBlank()) androidx.compose.ui.graphics.Color.Black else colors.muted)
                    }
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.heightIn(max = 480.dp),
                contentPadding = PaddingValues(bottom = 32.dp),
            ) {
                item {
                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { showCreate = true }
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        FirmiumIcon(Icons.Default.Add, contentDescription = null,
                            tint = colors.accent, modifier = Modifier.size(32.dp))
                        Spacer(Modifier.width(16.dp))
                        Text("New playlist", fontSize = 14.sp, fontFamily = FontFamily.Monospace,
                            color = colors.accent)
                    }
                    FirmiumDivider()
                }
                items(playlists, key = { it.id }) { playlist ->
                    Row(
                        modifier = Modifier.fillMaxWidth()
                            .clickable { onAddTo(playlist.id); onDismiss() }
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        FirmiumIcon(Icons.AutoMirrored.Filled.PlaylistPlay, contentDescription = null,
                            tint = colors.muted, modifier = Modifier.size(32.dp))
                        Spacer(Modifier.width(16.dp))
                        Column {
                            Text(playlist.name, fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.text)
                            Text("${playlist.tracks.size} tracks", fontSize = 12.sp,
                                fontFamily = FontFamily.Monospace, color = colors.muted)
                        }
                    }
                    FirmiumDivider()
                }
            }
        }
    }
}
