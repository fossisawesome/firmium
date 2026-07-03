package com.fossisawesome.firmium.ui.tv

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.db.PlayHistoryRepository
import com.fossisawesome.firmium.data.db.RecapStats
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

private val rangeOptions = listOf("7d" to "7 days", "30d" to "30 days", "3mo" to "3 months", "1y" to "1 year", "all" to "All time")

private fun rangeBounds(id: String): Pair<Long, Long> {
    val now = System.currentTimeMillis() / 1000
    val day = 86400L
    return when (id) {
        "7d" -> (now - 7 * day) to now
        "30d" -> (now - 30 * day) to now
        "3mo" -> (now - 90 * day) to now
        "1y" -> (now - 365 * day) to now
        else -> 0L to now
    }
}

private fun formatDuration(totalSeconds: Long): String {
    val h = totalSeconds / 3600
    val m = (totalSeconds % 3600) / 60
    return when {
        h > 0 -> "${h}h ${m}m"
        m > 0 -> "${m}m"
        else -> "${totalSeconds}s"
    }
}

private val DOW = listOf("Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat")

// Static list of stats — no pager/share-image capture (touch-oriented, not relevant on TV).
@Composable
fun TvRecapScreen(
    repository: PlayHistoryRepository,
    coverUrlFor: (String?) -> String?,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var rangeIndex by remember { mutableStateOf(1) }
    var stats by remember { mutableStateOf<RecapStats?>(null) }

    BackHandler { onBack() }

    LaunchedEffect(rangeIndex) {
        val (from, to) = rangeBounds(rangeOptions[rangeIndex].first)
        stats = repository.recap(from, to)
    }

    LazyColumn(modifier = Modifier.fillMaxSize().padding(48.dp), content = {
        item {
            Text(text = "Recap & Listening Stats", color = colors.text, fontSize = 24.sp, modifier = Modifier.padding(bottom = 16.dp))
            TvCycleRow(
                label = "Range",
                options = rangeOptions.map { it.second },
                selectedIndex = rangeIndex,
                colors = colors,
                onSelect = { rangeIndex = it },
                modifier = Modifier.padding(bottom = 24.dp),
            )

            val s = stats
            if (s == null) {
                Text(text = "Loading…", color = colors.muted, fontSize = 14.sp)
                return@item
            }

            Text(text = "${formatDuration(s.totalSeconds)} listened · ${s.totalPlays} tracks played", color = colors.text, fontSize = 16.sp, modifier = Modifier.padding(bottom = 24.dp))

            Text(text = "Top Tracks", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 8.dp))
            s.topTracks.forEach { t ->
                Text(text = "${t.title} — ${t.artist ?: ""} (${t.count})", color = colors.text, fontSize = 13.sp, modifier = Modifier.padding(bottom = 4.dp))
            }

            Text(text = "Top Artists", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp, bottom = 8.dp))
            s.topArtists.forEach { a ->
                Text(text = "${a.name} (${a.count})", color = colors.text, fontSize = 13.sp, modifier = Modifier.padding(bottom = 4.dp))
            }

            Text(text = "Top Albums", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp, bottom = 8.dp))
            s.topAlbums.forEach { a ->
                Text(text = "${a.name} — ${a.artist ?: ""} (${a.count})", color = colors.text, fontSize = 13.sp, modifier = Modifier.padding(bottom = 4.dp))
            }

            s.topGenre?.let { g ->
                Text(text = "Top genre: ${g.genre} (${g.count} plays)", color = colors.text, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp))
            }

            Text(text = "By day of week", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp, bottom = 8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
                s.byDayOfWeek.forEachIndexed { index, count ->
                    Column {
                        Text(text = DOW[index], color = colors.muted, fontSize = 11.sp)
                        Text(text = "$count", color = colors.text, fontSize = 13.sp)
                    }
                }
            }

            s.biggestDiscovery?.let { d ->
                Text(text = "Biggest discovery: ${d.title} — ${d.artist ?: ""} (${d.count} plays)", color = colors.text, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp))
            }

            Text(text = "Active days: ${s.daysActive} · Longest streak: ${s.longestStreak} days", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp))
        }
    })
}
