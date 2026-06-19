package com.fossisawesome.firmium.data.eq

/**
 * Minimal hand-rolled parser for the desktop's `eq.toml` (no TOML dependency).
 *
 * Targets exactly the shape the desktop writes via `toml::to_string_pretty`,
 * which uses array-of-tables for bands:
 *
 * ```
 * [profiles.warm]
 * type = "graphic"
 *
 * [[profiles.warm.bands]]
 * freq = 60.0
 * gain = 2.0
 *
 * [devices."Built-in Speakers"]
 * active_profile = "warm"
 * ```
 *
 * Hand-authored inline tables (`bands = [{ freq = 60, gain = 2.0 }]`) are also
 * accepted. `[settings]` and `[devices.*]` tables are ignored — only profiles
 * are imported.
 */
object TomlEqParser {

    private class Builder(val mode: String) {
        val bands = mutableListOf<EqBand>()
    }

    fun parse(text: String): List<EqProfile> {
        val builders = LinkedHashMap<String, Builder>()
        val modes = LinkedHashMap<String, String>()
        var currentProfile: String? = null
        // Context: "profile" (under [profiles.NAME]), "band" (under [[..bands]]), or "other".
        var context = "other"
        var currentBand: MutableMap<String, Float>? = null

        fun finishBand() {
            val name = currentProfile ?: return
            val band = currentBand ?: return
            val freq = band["freq"] ?: return
            val gain = band["gain"] ?: 0f
            builders[name]?.bands?.add(EqBand(freq, gain, band["q"]))
            currentBand = null
        }

        for (raw in text.lines()) {
            val line = raw.substringBefore('#').trim()
            if (line.isEmpty()) continue

            if (line.startsWith("[[") && line.endsWith("]]")) {
                finishBand()
                val segs = splitPath(line.substring(2, line.length - 2))
                if (segs.size == 3 && segs[0] == "profiles" && segs[2] == "bands") {
                    currentProfile = segs[1]
                    ensureProfile(builders, modes, segs[1])
                    context = "band"
                    currentBand = mutableMapOf()
                } else {
                    context = "other"
                }
                continue
            }

            if (line.startsWith("[") && line.endsWith("]")) {
                finishBand()
                val segs = splitPath(line.substring(1, line.length - 1))
                if (segs.size >= 2 && segs[0] == "profiles") {
                    currentProfile = segs[1]
                    ensureProfile(builders, modes, segs[1])
                    context = "profile"
                } else {
                    currentProfile = null
                    context = "other"
                }
                continue
            }

            val eq = line.indexOf('=')
            if (eq <= 0) continue
            val key = line.substring(0, eq).trim()
            val value = line.substring(eq + 1).trim()

            when (context) {
                "band" -> currentBand?.let { it[key] = value.toFloatOrNull() ?: return@let }
                "profile" -> {
                    val name = currentProfile ?: continue
                    when (key) {
                        "type" -> modes[name] = unquote(value)
                        "bands" -> parseInlineBands(value).forEach { builders[name]?.bands?.add(it) }
                    }
                }
            }
        }
        finishBand()

        return builders.map { (name, b) ->
            EqProfile(name, modes[name] ?: "graphic", b.bands.toList())
        }
    }

    private fun ensureProfile(
        builders: LinkedHashMap<String, Builder>,
        modes: LinkedHashMap<String, String>,
        name: String,
    ) {
        builders.getOrPut(name) { Builder(modes[name] ?: "graphic") }
    }

    /** Split a dotted TOML key path, respecting quoted segments. */
    private fun splitPath(path: String): List<String> {
        val out = mutableListOf<String>()
        val sb = StringBuilder()
        var inQuote = false
        for (c in path) {
            when {
                c == '"' -> inQuote = !inQuote
                c == '.' && !inQuote -> { out.add(sb.toString().trim()); sb.clear() }
                else -> sb.append(c)
            }
        }
        out.add(sb.toString().trim())
        return out.map { it.trim() }
    }

    private fun unquote(s: String): String = s.trim().trim('"')

    /** Parse `[{ freq = 60, gain = 2.0 }, { freq = 1000, gain = -3, q = 1.4 }]`. */
    private fun parseInlineBands(value: String): List<EqBand> {
        val result = mutableListOf<EqBand>()
        var depth = 0
        val current = StringBuilder()
        for (c in value) {
            when (c) {
                '{' -> { depth++; if (depth == 1) current.clear() }
                '}' -> {
                    if (depth == 1) {
                        parseInlineBand(current.toString())?.let { result.add(it) }
                    }
                    depth--
                }
                else -> if (depth >= 1) current.append(c)
            }
        }
        return result
    }

    private fun parseInlineBand(body: String): EqBand? {
        val fields = HashMap<String, Float>()
        for (pair in body.split(',')) {
            val parts = pair.split('=')
            if (parts.size != 2) continue
            val k = parts[0].trim()
            val v = parts[1].trim().toFloatOrNull() ?: continue
            fields[k] = v
        }
        val freq = fields["freq"] ?: return null
        return EqBand(freq, fields["gain"] ?: 0f, fields["q"])
    }
}
