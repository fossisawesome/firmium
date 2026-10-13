package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Offer to continue the queue another client saved on the server. Styled like AccountDialog.
@Composable
fun ResumeQueueDialog(
    remote: ApiClient.RemotePlayQueue,
    onResume: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val font = LocalAppFontFamily.current
    val first = remote.songs.firstOrNull { it.id == remote.currentId } ?: remote.songs.first()

    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(4.dp))
                .background(colors.surface)
                .border(1.dp, colors.border, RoundedCornerShape(4.dp))
                .padding(24.dp),
        ) {
            Text(
                text = "Resume server queue?",
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = font,
                color = colors.text,
                modifier = Modifier.padding(bottom = 8.dp),
            )
            Text(
                text = "${remote.songs.size} tracks saved by ${remote.changedBy ?: "another device"}.",
                fontSize = 12.sp,
                fontFamily = font,
                color = colors.muted,
            )
            Text(
                text = "Starts at ${first.title}",
                fontSize = 12.sp,
                fontFamily = font,
                color = colors.muted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(bottom = 20.dp),
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                DialogButton("Not now", colors.muted, Modifier.weight(1f), onDismiss)
                DialogButton("Resume", colors.accent, Modifier.weight(1f), onResume)
            }
        }
    }
}

@Composable
private fun DialogButton(
    label: String,
    color: androidx.compose.ui.graphics.Color,
    modifier: Modifier,
    onClick: () -> Unit,
) {
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(2.dp))
            .border(1.dp, color, RoundedCornerShape(2.dp))
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onClick)
            .padding(vertical = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            fontSize = 12.sp,
            fontWeight = FontWeight.Bold,
            fontFamily = LocalAppFontFamily.current,
            color = color,
            letterSpacing = 0.5.sp,
        )
    }
}
