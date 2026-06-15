package com.fossisawesome.firmium.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color

// One entry per theme — id matches the key stored in AppPreferences.
data class FirmiumTheme(
    val id: String,
    val name: String,
    val isDark: Boolean,
    val bg: Color,
    val surface: Color,
    val surface2: Color,
    val text: Color,
    val muted: Color,
    val accent: Color,
    val error: Color,
)

private fun hex(s: String): Color {
    val v = s.trimStart('#')
    return Color(android.graphics.Color.parseColor("#$v"))
}

// All themes ported from the TOML files in themes/.
val ALL_THEMES: List<FirmiumTheme> = listOf(
    FirmiumTheme("firmium", "Firmium", true,
        bg = hex("0f0f0f"), surface = hex("1a1a1a"), surface2 = hex("242424"),
        text = hex("f0f0f0"), muted = hex("888888"), accent = hex("e8c97e"), error = hex("e06060")),
    FirmiumTheme("dracula", "Dracula", true,
        bg = hex("282a36"), surface = hex("343746"), surface2 = hex("44475a"),
        text = hex("f8f8f2"), muted = hex("6272a4"), accent = hex("bd93f9"), error = hex("ff5555")),
    FirmiumTheme("tokyo-night", "Tokyo Night", true,
        bg = hex("1a1b26"), surface = hex("2b2d3a"), surface2 = hex("3c3f50"),
        text = hex("c0caf5"), muted = hex("7aa2f7"), accent = hex("9ece6a"), error = hex("f7768e")),
    FirmiumTheme("catppuccin-mocha", "Catppuccin Mocha", true,
        bg = hex("1e1e2e"), surface = hex("313244"), surface2 = hex("45475a"),
        text = hex("cdd6f4"), muted = hex("a6adc8"), accent = hex("a6e3a1"), error = hex("f38ba8")),
    FirmiumTheme("catppuccin-frappe", "Catppuccin Frappé", true,
        bg = hex("303446"), surface = hex("414559"), surface2 = hex("51576d"),
        text = hex("c6d0f5"), muted = hex("a5adce"), accent = hex("a6d189"), error = hex("e78284")),
    FirmiumTheme("catppuccin-macchiato", "Catppuccin Macchiato", true,
        bg = hex("24273a"), surface = hex("363a4f"), surface2 = hex("494d64"),
        text = hex("cad3f5"), muted = hex("a5adcb"), accent = hex("a6da95"), error = hex("ed8796")),
    FirmiumTheme("gruvbox", "Gruvbox", true,
        bg = hex("282828"), surface = hex("3c3836"), surface2 = hex("504945"),
        text = hex("ebdbb2"), muted = hex("928374"), accent = hex("b8bb26"), error = hex("fb4934")),
    FirmiumTheme("nord", "Nord", true,
        bg = hex("2e3440"), surface = hex("3b4252"), surface2 = hex("434c5e"),
        text = hex("eceff4"), muted = hex("81a1c1"), accent = hex("a3be8c"), error = hex("bf616a")),
    FirmiumTheme("synthwave", "Synthwave '84", true,
        bg = hex("262335"), surface = hex("2a2139"), surface2 = hex("34294f"),
        text = hex("ffffff"), muted = hex("848bbd"), accent = hex("ff7edb"), error = hex("fe4450")),
    FirmiumTheme("ayu", "Ayu Dark", true,
        bg = hex("0d1017"), surface = hex("131721"), surface2 = hex("1c2333"),
        text = hex("bfbdb6"), muted = hex("5c6773"), accent = hex("ffb454"), error = hex("f07178")),
    FirmiumTheme("github-dark", "GitHub Dark", true,
        bg = hex("0d1117"), surface = hex("161b22"), surface2 = hex("21262d"),
        text = hex("e6edf3"), muted = hex("7d8590"), accent = hex("2f81f7"), error = hex("f85149")),
    FirmiumTheme("adwaita-dark", "Adwaita Dark", true,
        bg = hex("242424"), surface = hex("303030"), surface2 = hex("3c3c3c"),
        text = hex("deddda"), muted = hex("9a9996"), accent = hex("3584e4"), error = hex("e01b24")),
    FirmiumTheme("nordfox", "Nordfox", true,
        bg = hex("232831"), surface = hex("2e3440"), surface2 = hex("3b4252"),
        text = hex("cdcecf"), muted = hex("60728a"), accent = hex("81a1c1"), error = hex("bf616a")),
    FirmiumTheme("monokai", "Monokai Classic", true,
        bg = hex("272822"), surface = hex("3e3d32"), surface2 = hex("49483e"),
        text = hex("f8f8f2"), muted = hex("75715e"), accent = hex("a6e22e"), error = hex("f92672")),
    // Light themes
    FirmiumTheme("adwaita", "Adwaita", false,
        bg = hex("fafafa"), surface = hex("ffffff"), surface2 = hex("f0f0f0"),
        text = hex("2e3436"), muted = hex("8e9399"), accent = hex("3584e4"), error = hex("e01b24")),
    FirmiumTheme("catppuccin-latte", "Catppuccin Latte", false,
        bg = hex("eff1f5"), surface = hex("ffffff"), surface2 = hex("e6e9ef"),
        text = hex("4c4f69"), muted = hex("8c8fa1"), accent = hex("40a02b"), error = hex("d20f39")),
    FirmiumTheme("ayu-light", "Ayu Light", false,
        bg = hex("fafafa"), surface = hex("ffffff"), surface2 = hex("f0f0f0"),
        text = hex("5c6166"), muted = hex("abb0b6"), accent = hex("ff9940"), error = hex("f51818")),
)

val DEFAULT_THEME_ID = "firmium"

fun themeById(id: String): FirmiumTheme =
    ALL_THEMES.find { it.id == id } ?: ALL_THEMES.first()

// Provides FirmiumColors and isDark flag to the entire composable tree via CompositionLocals.
// No MaterialTheme — all color tokens are accessed via LocalFirmiumColors.current.
@Composable
fun FirmiumTheme(
    themeId: String = DEFAULT_THEME_ID,
    content: @Composable () -> Unit,
) {
    val theme = remember(themeId) { themeById(themeId) }
    CompositionLocalProvider(
        LocalFirmiumColors provides remember(theme) { theme.toFirmiumColors() },
        LocalFirmiumIsDark provides theme.isDark,
        content = content,
    )
}
