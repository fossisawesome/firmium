package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Mic
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
import com.fossisawesome.firmium.data.db.PodcastChannelEntity
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

@Composable
fun PodcastsScreen(
    channels: List<PodcastChannelEntity>,
    addError: String?,
    onChannelClick: (String) -> Unit,
    onAddChannel: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var showAddDialog by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 10.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "Podcasts".uppercase(),
                fontSize = 12.sp, fontFamily = FontFamily.Monospace,
                color = colors.muted, letterSpacing = 1.sp,
                modifier = Modifier.weight(1f),
            )
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(3.dp))
                    .background(colors.surface2)
                    .border(1.dp, colors.accent.copy(alpha = 0.5f), RoundedCornerShape(3.dp))
                    .clickable { showAddDialog = true }
                    .padding(horizontal = 14.dp, vertical = 6.dp),
            ) {
                Text("+ Add", fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.accent)
            }
        }

        if (channels.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(
                    "No podcasts yet. Add one by RSS feed URL.",
                    fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = colors.muted,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                items(channels, key = { it.id }) { channel ->
                    PodcastChannelRow(channel = channel, onClick = { onChannelClick(channel.id) })
                    FirmiumDivider()
                }
            }
        }
    }

    if (showAddDialog) {
        AddPodcastDialog(
            error = addError,
            onConfirm = { url ->
                if (url.isNotBlank()) {
                    onAddChannel(url)
                    showAddDialog = false
                }
            },
            onDismiss = { showAddDialog = false },
        )
    }
}

@Composable
private fun PodcastChannelRow(channel: PodcastChannelEntity, onClick: () -> Unit) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }
            .padding(horizontal = 10.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            modifier = Modifier.size(48.dp).clip(RoundedCornerShape(8.dp)).background(colors.surface2),
            contentAlignment = Alignment.Center,
        ) {
            FirmiumIcon(Icons.Default.Mic, contentDescription = null, tint = colors.muted, modifier = Modifier.size(22.dp))
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(
                channel.title, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace,
                fontSize = 14.sp, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            Text(
                channel.description.orEmpty(),
                fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
                maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun AddPodcastDialog(
    error: String?,
    onConfirm: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var url by remember { mutableStateOf("") }

    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(8.dp))
                .background(colors.surface)
                .border(1.dp, colors.border, RoundedCornerShape(8.dp))
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                "Add a podcast", fontSize = 16.sp, fontWeight = FontWeight.Bold,
                fontFamily = FontFamily.Monospace, color = colors.text,
            )
            FirmiumTextField(
                value = url,
                onValueChange = { url = it },
                placeholder = "RSS feed URL",
                modifier = Modifier.fillMaxWidth(),
            )
            if (error != null) {
                Text(error, fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.error)
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End),
            ) {
                Text(
                    "Cancel", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
                    modifier = Modifier.clickable { onDismiss() }.padding(8.dp),
                )
                Text(
                    "Add", fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                    color = if (url.isNotBlank()) colors.accent else colors.muted,
                    modifier = Modifier
                        .clickable(enabled = url.isNotBlank()) { onConfirm(url) }
                        .padding(8.dp),
                )
            }
        }
    }
}
