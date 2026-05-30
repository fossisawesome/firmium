package com.fossisawesome.firmium.ui.theme

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

// Typed color tokens exposed to every composable via CompositionLocal — replaces MaterialTheme.colorScheme.
data class FirmiumColors(
    val bg: Color,
    val surface: Color,
    val surface2: Color,
    val text: Color,
    val muted: Color,
    val accent: Color,
    val error: Color,
) {
    val border: Color get() = surface2.copy(alpha = 0.4f)
}

val LocalFirmiumColors = staticCompositionLocalOf<FirmiumColors> {
    error("No FirmiumTheme provided")
}

val LocalFirmiumIsDark = staticCompositionLocalOf { true }

fun FirmiumTheme.toFirmiumColors() = FirmiumColors(
    bg = bg, surface = surface, surface2 = surface2,
    text = text, muted = muted, accent = accent, error = error,
)
