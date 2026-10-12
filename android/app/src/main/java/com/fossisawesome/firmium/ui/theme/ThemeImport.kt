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

// Drops a trailing `# comment`, but keeps `#` inside quotes (hex colors like "#242424").
private fun stripComment(line: String): String {
    var quoted = false
    for ((i, c) in line.withIndex()) {
        if (c == '"') quoted = !quoted
        else if (c == '#' && !quoted) return line.substring(0, i)
    }
    return line
}

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
        val line = stripComment(raw).trim()
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

// Accepts #RGB, #RRGGBB, CSS-style #RRGGBBAA and rgb()/rgba(), which desktop theme files use.
private fun parseCssColor(raw: String): Color? {
    val v = raw.trim()
    if (v.startsWith("rgb", ignoreCase = true)) {
        val p = v.substringAfter('(').substringBefore(')').split(',').map { it.trim() }
        if (p.size < 3) return null
        val ch = p.take(3).map { it.toFloatOrNull()?.toInt()?.coerceIn(0, 255) ?: return null }
        val a = p.getOrNull(3)?.toFloatOrNull()?.coerceIn(0f, 1f) ?: 1f
        return Color(ch[0], ch[1], ch[2], (a * 255).toInt())
    }
    val h = v.trimStart('#')
    val full = if (h.length == 8) h.substring(6) + h.substring(0, 6) else h
    return runCatching { hex(full) }.getOrNull()
}

/**
 * Build a FirmiumTheme from a parsed file. Only bg, text and accent are required;
 * surface, surface2, muted and error fall back to values derived from those.
 * Returns the theme, or the list of required keys that are missing/invalid.
 */
private fun toFirmiumTheme(id: String, parsed: ParsedTheme, sourceFile: String): Pair<FirmiumTheme?, List<String>> {
    fun color(key: String): Color? = parsed.colors[key]?.let { parseCssColor(it) }
    val bg = color("bg")
    val text = color("text")
    val accent = color("accent")
    val bad = listOfNotNull("bg".takeIf { bg == null }, "text".takeIf { text == null }, "accent".takeIf { accent == null })
    if (bg == null || text == null || accent == null) return null to bad
    val surface = color("surface") ?: bg
    return FirmiumTheme(
        id = id,
        name = parsed.name,
        isDark = parsed.colorScheme != "light",
        bg = bg,
        surface = surface,
        surface2 = color("surface2") ?: surface,
        text = text,
        muted = color("muted") ?: text.copy(alpha = 0.6f),
        accent = accent,
        error = color("error") ?: Color(0xFFE53935),
        isImported = true,
        sourceFile = sourceFile,
    ) to emptyList()
}

/** Read all valid imported themes from filesDir/themes/. Invalid files are skipped. */
fun loadImportedThemes(context: Context): List<FirmiumTheme> {
    val dir = themesDir(context)
    val files = dir.listFiles { f -> f.isFile && f.name.endsWith(".toml") } ?: return emptyList()
    return files.mapNotNull { file ->
        val content = runCatching { file.readText() }.getOrNull() ?: return@mapNotNull null
        val parsed = parseThemeToml(content) ?: return@mapNotNull null
        toFirmiumTheme(id = file.nameWithoutExtension, parsed = parsed, sourceFile = file.name).first
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
    val missing = toFirmiumTheme(sanitizeName(parsed.name), parsed, "tmp").second
    if (missing.isNotEmpty()) {
        return Result.failure(Exception("Theme is missing or has invalid colors: ${missing.joinToString(", ")}"))
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
