package com.fossisawesome.firmium.ui.theme

import android.content.Context
import android.net.Uri
import androidx.compose.ui.graphics.Color
import java.io.File

// Imported themes live as one .toml per theme under filesDir/themes/. The format
// matches the desktop theme files (name, color_scheme, [colors] table), so a file
// authored for desktop imports unchanged here.

private const val THEMES_DIR = "themes"
private const val MAX_THEME_BYTES = 50 * 1024

/** Parsed shape of a theme .toml — only the fields the Android FirmiumTheme needs. */
private data class ParsedTheme(
    val name: String,
    val colorScheme: String?,
    val colors: Map<String, String>,
)

private fun themesDir(context: Context): File = File(context.filesDir, THEMES_DIR)

/**
 * Minimal hand-rolled parser for the theme .toml format (no TOML dependency, mirrors
 * the approach in TomlEqParser). Reads top-level `name`/`color_scheme` and the keys
 * under a `[colors]` table. Returns null if it can't read a usable theme.
 */
private fun parseThemeToml(text: String): ParsedTheme? {
    var name: String? = null
    var colorScheme: String? = null
    val colors = mutableMapOf<String, String>()
    var inColors = false

    for (raw in text.lines()) {
        val line = raw.substringBefore('#').trim()
        if (line.isEmpty()) continue

        if (line.startsWith("[") && line.endsWith("]")) {
            inColors = line.substring(1, line.length - 1).trim() == "colors"
            continue
        }

        val eq = line.indexOf('=')
        if (eq <= 0) continue
        val key = line.substring(0, eq).trim()
        val value = line.substring(eq + 1).trim().trim('"')

        if (inColors) {
            colors[key] = value
        } else when (key) {
            "name" -> name = value
            "color_scheme" -> colorScheme = value
        }
    }

    val n = name?.trim().orEmpty()
    if (n.isEmpty()) return null
    return ParsedTheme(n, colorScheme, colors)
}

/** Build a FirmiumTheme from a parsed file, or null if a required color is missing/invalid. */
private fun toFirmiumTheme(id: String, parsed: ParsedTheme, sourceFile: String): FirmiumTheme? {
    fun color(key: String): Color? = parsed.colors[key]?.let {
        runCatching { hex(it) }.getOrNull()
    }
    return FirmiumTheme(
        id = id,
        name = parsed.name,
        isDark = parsed.colorScheme != "light",
        bg = color("bg") ?: return null,
        surface = color("surface") ?: return null,
        surface2 = color("surface2") ?: return null,
        text = color("text") ?: return null,
        muted = color("muted") ?: return null,
        accent = color("accent") ?: return null,
        error = color("error") ?: return null,
        isImported = true,
        sourceFile = sourceFile,
    )
}

/** Read all valid imported themes from filesDir/themes/. Invalid files are skipped. */
fun loadImportedThemes(context: Context): List<FirmiumTheme> {
    val dir = themesDir(context)
    val files = dir.listFiles { f -> f.isFile && f.name.endsWith(".toml") } ?: return emptyList()
    return files.mapNotNull { file ->
        val content = runCatching { file.readText() }.getOrNull() ?: return@mapNotNull null
        val parsed = parseThemeToml(content) ?: return@mapNotNull null
        toFirmiumTheme(id = file.nameWithoutExtension, parsed = parsed, sourceFile = file.name)
    }.sortedBy { it.name.lowercase() }
}

/** Built-in themes plus any imported ones (imported appended after built-ins). */
fun allThemes(context: Context): List<FirmiumTheme> = ALL_THEMES + loadImportedThemes(context)

private fun sanitizeName(name: String): String {
    val cleaned = name.lowercase().map { c -> if (c.isLetterOrDigit()) c else '-' }.joinToString("")
        .trim('-').replace(Regex("-+"), "-")
    return cleaned.ifEmpty { "theme" }
}

/**
 * Validate and copy the picked file into filesDir/themes/<sanitized-name>.toml.
 * Returns failure with a user-facing message on any error; overwrites a same-name file.
 */
fun importThemeFromUri(context: Context, uri: Uri): Result<Unit> {
    val bytes = runCatching {
        context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
    }.getOrNull() ?: return Result.failure(Exception("Couldn't read the selected file"))

    if (bytes.size > MAX_THEME_BYTES) {
        return Result.failure(Exception("File is too large (max 50 KB)"))
    }

    val text = runCatching { bytes.toString(Charsets.UTF_8) }.getOrNull()
        ?: return Result.failure(Exception("File isn't valid text"))
    val parsed = parseThemeToml(text)
        ?: return Result.failure(Exception("Not a valid theme file (missing name or colors)"))
    // Confirm the colors actually build a theme before saving.
    if (toFirmiumTheme(sanitizeName(parsed.name), parsed, "tmp") == null) {
        return Result.failure(Exception("Theme is missing one or more required colors"))
    }

    val dir = themesDir(context)
    if (!dir.exists() && !dir.mkdirs()) {
        return Result.failure(Exception("Couldn't create the themes folder"))
    }
    val target = File(dir, "${sanitizeName(parsed.name)}.toml")
    return runCatching { target.writeText(text); Unit }
        .recoverCatching { throw Exception("Couldn't save the theme") }
}

/** Delete an imported theme by its filename under filesDir/themes/. */
fun deleteImportedTheme(context: Context, filename: String) {
    runCatching { File(themesDir(context), filename).delete() }
}
