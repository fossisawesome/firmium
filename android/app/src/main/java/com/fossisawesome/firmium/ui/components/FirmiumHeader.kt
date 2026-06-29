package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Flat detail-screen header matching .mobile-page-header: back arrow left, title, optional action right.
// No elevation, bg = background colour, 1dp border-bottom.
// windowInsetsPadding(statusBars) is applied here so detail screens always clear the notch/status bar.
@Composable
fun FirmiumDetailHeader(
    title: String,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
    action: @Composable (() -> Unit)? = null,
) {
    val colors = LocalFirmiumColors.current

    Column(modifier = modifier.fillMaxWidth().background(colors.bg).windowInsetsPadding(WindowInsets.statusBars)) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 4.dp, end = if (action != null) 4.dp else 16.dp, top = 10.dp, bottom = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Back button — 44dp tap target, matches .mobile-header-btn
            FirmiumIconButton(onClick = onBack, modifier = Modifier.size(44.dp)) {
                FirmiumIcon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back",
                    tint = colors.muted)
            }
            Text(
                text = title,
                fontSize = 18.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current,
                color = colors.text,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            if (action != null) action()
        }
        FirmiumDivider(color = colors.border)
    }
}
