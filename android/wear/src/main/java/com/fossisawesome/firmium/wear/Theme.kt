package com.fossisawesome.firmium.wear

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.wear.compose.material.Colors
import androidx.wear.compose.material.MaterialTheme

private val FirmiumWearColors = Colors(
    primary = Color(0xFFB794F6),
    onPrimary = Color(0xFF1A1126),
)

@Composable
fun FirmiumWearTheme(content: @Composable () -> Unit) {
    MaterialTheme(colors = FirmiumWearColors, content = content)
}
