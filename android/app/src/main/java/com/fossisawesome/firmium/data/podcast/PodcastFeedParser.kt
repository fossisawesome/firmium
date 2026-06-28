package com.fossisawesome.firmium.data.podcast

import android.util.Xml
import org.xmlpull.v1.XmlPullParser
import java.io.InputStream
import java.text.SimpleDateFormat
import java.util.Locale

data class ParsedFeed(
    val title: String,
    val description: String?,
    val imageUrl: String?,
    val episodes: List<ParsedEpisode>,
)

data class ParsedEpisode(
    val guid: String,
    val title: String,
    val description: String?,
    val audioUrl: String,
    val durationSeconds: Long?,
    val publishedAt: Long?,
)

/**
 * Hand-rolled RSS 2.0 + iTunes-namespace parser using Android's built-in
 * XmlPullParser — matches AntennaPod's approach of avoiding a third-party RSS
 * library dependency for this.
 */
object PodcastFeedParser {

    private val rfc822Formats = listOf(
        SimpleDateFormat("EEE, dd MMM yyyy HH:mm:ss Z", Locale.US),
        SimpleDateFormat("dd MMM yyyy HH:mm:ss Z", Locale.US),
    )

    fun parse(input: InputStream): ParsedFeed {
        val parser = Xml.newPullParser()
        parser.setFeature(XmlPullParser.FEATURE_PROCESS_NAMESPACES, false)
        parser.setInput(input, null)

        var channelTitle = "Untitled Podcast"
        var channelDescription: String? = null
        var channelImageUrl: String? = null
        val episodes = mutableListOf<ParsedEpisode>()

        var inItem = false
        var itemGuid: String? = null
        var itemTitle: String? = null
        var itemDescription: String? = null
        var itemAudioUrl: String? = null
        var itemDuration: Long? = null
        var itemPublished: Long? = null

        var eventType = parser.eventType
        while (eventType != XmlPullParser.END_DOCUMENT) {
            when (eventType) {
                XmlPullParser.START_TAG -> when (parser.name) {
                    "item" -> {
                        inItem = true
                        itemGuid = null; itemTitle = null; itemDescription = null
                        itemAudioUrl = null; itemDuration = null; itemPublished = null
                    }
                    "title" -> {
                        val t = readText(parser)
                        if (inItem) itemTitle = t else channelTitle = t
                    }
                    "description" -> {
                        val d = readText(parser)
                        if (inItem) itemDescription = d else channelDescription = d
                    }
                    "guid" -> if (inItem) itemGuid = readText(parser)
                    "enclosure" -> if (inItem) itemAudioUrl = parser.getAttributeValue(null, "url")
                    "itunes:duration" -> if (inItem) itemDuration = parseItunesDuration(readText(parser))
                    "itunes:image" -> if (!inItem) channelImageUrl = parser.getAttributeValue(null, "href")
                    "pubDate" -> if (inItem) itemPublished = parseRfc822Date(readText(parser))
                }
                XmlPullParser.END_TAG -> if (parser.name == "item") {
                    inItem = false
                    val audioUrl = itemAudioUrl
                    val title = itemTitle
                    if (audioUrl != null && title != null) {
                        episodes.add(
                            ParsedEpisode(
                                guid = itemGuid ?: audioUrl,
                                title = title,
                                description = itemDescription,
                                audioUrl = audioUrl,
                                durationSeconds = itemDuration,
                                publishedAt = itemPublished,
                            ),
                        )
                    }
                }
            }
            eventType = parser.next()
        }

        return ParsedFeed(channelTitle, channelDescription, channelImageUrl, episodes)
    }

    private fun readText(parser: XmlPullParser): String {
        var result = ""
        if (parser.next() == XmlPullParser.TEXT) {
            result = parser.text
            parser.nextTag()
        }
        return result.trim()
    }

    /** Accepts `HH:MM:SS`, `MM:SS`, or a bare seconds integer (all valid per the iTunes podcast spec). */
    private fun parseItunesDuration(raw: String): Long? {
        if (raw.isBlank()) return null
        val parts = raw.split(":").mapNotNull { it.toLongOrNull() }
        return when (parts.size) {
            1 -> parts[0]
            2 -> parts[0] * 60 + parts[1]
            3 -> parts[0] * 3600 + parts[1] * 60 + parts[2]
            else -> null
        }
    }

    private fun parseRfc822Date(raw: String): Long? {
        for (format in rfc822Formats) {
            try {
                return format.parse(raw)?.time?.div(1000)
            } catch (_: Exception) {
                // try next format
            }
        }
        return null
    }
}
