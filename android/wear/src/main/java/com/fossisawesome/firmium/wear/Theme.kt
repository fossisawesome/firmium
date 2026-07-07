package com.fossisawesome.firmium.wear

import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.wear.compose.material.Colors
import androidx.wear.compose.material.MaterialTheme

// Fallback used until the phone has synced at least once (fresh install, or a watch that's
// never been in range of the phone since reinstalling).
private val DefaultWearColors = Colors(
    primary = Color(0xFFB794F6),
    onPrimary = Color(0xFF1A1126),
)

@Composable
fun FirmiumWearTheme(content: @Composable () -> Unit) {
    val app = LocalContext.current.applicationContext as FirmiumWearApplication
    val theme by app.watchPreferences.themeColors.collectAsState(initial = null)

    val colors = theme?.let { t ->
        val onAccent = if (t.isDark) Color.Black else Color.White
        Colors(
            primary = parseHexColor(t.accent),
            primaryVariant = parseHexColor(t.accent),
            secondary = parseHexColor(t.accent),
            background = parseHexColor(t.bg),
            surface = parseHexColor(t.surface),
            error = parseHexColor(t.error),
            onPrimary = onAccent,
            onSecondary = onAccent,
            onBackground = parseHexColor(t.text),
            onSurface = parseHexColor(t.text),
            onError = onAccent,
        )
    } ?: DefaultWearColors

    MaterialTheme(colors = colors, content = content)
}

private fun parseHexColor(hex: String): Color = Color(android.graphics.Color.parseColor(hex))
