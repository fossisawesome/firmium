package com.fossisawesome.firmium.ui.theme

// Maps the user-facing font display names (Settings > Appearance) to a key.
// Theme.kt resolves each key to an actual Compose FontFamily (needs R.font,
// so that step isn't unit-testable here) and applies it live via recomposition.
enum class AppFontKey {
    INTER, LIBERATION_MONO, MONOSPACE, DEFAULT, COMIC_SANS, SANS_SERIF,
    BIGBLUE_TERMINAL, COUSINE, FIRACODE, HACK,
}

val FONT_OPTIONS: List<String> = listOf(
    "Inter", "Liberation Mono", "Monospace", "System", "Iced", "Comic Sans",
    "Sans Serif", "BigBlue Terminal", "Cousine", "FiraCode", "Hack",
)

fun fontKeyFor(displayName: String): AppFontKey = when (displayName) {
    "Inter" -> AppFontKey.INTER
    "Liberation Mono" -> AppFontKey.LIBERATION_MONO
    "Monospace" -> AppFontKey.MONOSPACE
    "Comic Sans" -> AppFontKey.COMIC_SANS
    "Sans Serif" -> AppFontKey.SANS_SERIF
    "BigBlue Terminal" -> AppFontKey.BIGBLUE_TERMINAL
    "Cousine" -> AppFontKey.COUSINE
    "FiraCode" -> AppFontKey.FIRACODE
    "Hack" -> AppFontKey.HACK
    else -> AppFontKey.DEFAULT // "System", "Iced", and any unrecognized value
}
