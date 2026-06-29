package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.UserError
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// App-wide error notification card pinned to the bottom-center of its Box parent.
// Custom (no Material3) — surface-tinted card with an error-coloured accent strip;
// tap anywhere or the close icon to dismiss. Renders nothing when error is null.
@Composable
fun ErrorHost(
    error: UserError?,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = LocalFirmiumColors.current
    // Keep the last message visible during the exit animation, after error goes null.
    var lastMessage by remember { mutableStateOf("") }
    if (error != null) lastMessage = error.message

    AnimatedVisibility(
        visible = error != null,
        enter = fadeIn() + slideInVertically { it / 2 },
        exit = fadeOut() + slideOutVertically { it / 2 },
        modifier = modifier,
    ) {
        Row(
            modifier = Modifier
                .padding(12.dp)
                .clip(RoundedCornerShape(8.dp))
                .background(colors.surface)
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                ) { onDismiss() },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier = Modifier
                    .width(3.dp)
                    .height(40.dp)
                    .background(colors.error),
            )
            Text(
                text = lastMessage,
                color = colors.text,
                fontSize = 13.sp,
                fontFamily = LocalAppFontFamily.current,
                modifier = Modifier
                    .weight(1f)
                    .padding(horizontal = 12.dp, vertical = 12.dp),
            )
            FirmiumIconButton(
                onClick = onDismiss,
                modifier = Modifier.size(40.dp).padding(end = 4.dp),
            ) {
                FirmiumIcon(
                    Icons.Default.Close,
                    contentDescription = "Dismiss",
                    tint = colors.muted,
                    modifier = Modifier.size(18.dp),
                )
            }
        }
    }
}
