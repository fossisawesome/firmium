package com.fossisawesome.firmium.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Share
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.db.DiscoveryStat
import com.fossisawesome.firmium.data.db.PlayHistoryRepository
import com.fossisawesome.firmium.data.db.RecapStats
import com.fossisawesome.firmium.ui.ShareUtils
import com.fossisawesome.firmium.ui.components.CoverImage
import com.fossisawesome.firmium.ui.components.FirmiumIcon
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private data class RangeOption(val id: String, val label: String)

private val RANGES = listOf(
    RangeOption("7d", "7 days"),
    RangeOption("30d", "30 days"),
    RangeOption("3mo", "3 months"),
    RangeOption("1y", "1 year"),
    RangeOption("all", "All time"),
)

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

@Composable
fun RecapScreen(
    repository: PlayHistoryRepository,
    coverUrlFor: (String?) -> String?,
    onClose: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    BackHandler { onClose() }

    var range by remember { mutableStateOf("30d") }
    var stats by remember { mutableStateOf<RecapStats?>(null) }
    var loading by remember { mutableStateOf(true) }

    LaunchedEffect(range) {
        loading = true
        val (from, to) = rangeBounds(range)
        stats = repository.recap(from, to)
        loading = false
    }

    // One graphics layer over the pager — captures whichever card is currently shown.
    val captureLayer = rememberGraphicsLayer()
    val pageCount = 10
    val pagerState = rememberPagerState(pageCount = { pageCount })

    fun shareCurrentCard() {
        scope.launch {
            val bitmap = captureLayer.toImageBitmap().asAndroidBitmap()
            ShareUtils.shareBitmap(context, "firmium-recap.png", bitmap)
        }
    }

    Column(modifier = Modifier.fillMaxSize().background(colors.bg)) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("Firmium Recap", fontSize = 18.sp, fontWeight = FontWeight.Bold,
                fontFamily = FontFamily.Monospace, color = colors.text, modifier = Modifier.weight(1f))
            CircleIconButton(Icons.Default.Share, "Share", colors.text) { shareCurrentCard() }
            Spacer(Modifier.width(8.dp))
            CircleIconButton(Icons.Default.Close, "Close", colors.text) { onClose() }
        }

        // Range selector
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            RANGES.forEach { opt ->
                val active = range == opt.id
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(50))
                        .background(if (active) colors.accent else colors.surface)
                        .clickable { range = opt.id }
                        .padding(horizontal = 12.dp, vertical = 6.dp),
                ) {
                    Text(opt.label, fontSize = 12.sp, fontFamily = FontFamily.Monospace,
                        color = if (active) colors.bg else colors.muted)
                }
            }
        }

        val s = stats
        when {
            loading -> CenterMessage("Crunching your listening…")
            s == null || s.totalPlays == 0 -> CenterMessage("No plays recorded in this range yet.")
            else -> {
                HorizontalPager(
                    state = pagerState,
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .drawWithContent {
                            captureLayer.record { this@drawWithContent.drawContent() }
                            drawLayer(captureLayer)
                        },
                ) { page ->
                    Box(modifier = Modifier.fillMaxSize().background(colors.bg)) {
                        when (page) {
                            0 -> HeroCard("You listened for", formatDuration(s.totalSeconds), "${s.totalPlays} tracks played")
                            1 -> TrackListCard("Top Tracks", s.topTracks, coverUrlFor)
                            2 -> ArtistListCard("Top Artists", s.topArtists)
                            3 -> AlbumListCard("Top Albums", s.topAlbums, coverUrlFor)
                            4 -> HeroCard("Your sound was", s.topGenre?.genre ?: "—",
                                s.topGenre?.let { "${it.count} plays" } ?: "No genre data", accent = true)
                            5 -> TimeOfDayCard(s)
                            6 -> DayOfWeekCard(s)
                            7 -> DiscoveryCard(s.biggestDiscovery, coverUrlFor)
                            8 -> HeroCard("Longest streak", "${s.longestStreak} days",
                                "${s.daysActive} days with music in this range")
                            else -> SummaryCard(s)
                        }
                    }
                }

                // Page dots
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .windowInsetsPadding(WindowInsets.navigationBars)
                        .padding(vertical = 12.dp),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    repeat(pageCount) { i ->
                        Box(
                            modifier = Modifier
                                .padding(horizontal = 4.dp)
                                .size(8.dp)
                                .clip(CircleShape)
                                .background(if (i == pagerState.currentPage) colors.accent else colors.surface2),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun CircleIconButton(icon: androidx.compose.ui.graphics.vector.ImageVector, desc: String, tint: Color, onClick: () -> Unit) {
    val colors = LocalFirmiumColors.current
    Box(
        modifier = Modifier
            .size(36.dp)
            .clip(CircleShape)
            .background(colors.surface)
            .clickable { onClick() },
        contentAlignment = Alignment.Center,
    ) {
        FirmiumIcon(icon, contentDescription = desc, tint = tint, modifier = Modifier.size(16.dp))
    }
}

@Composable
private fun ColumnScope.CenterMessage(text: String) {
    val colors = LocalFirmiumColors.current
    Box(modifier = Modifier.weight(1f).fillMaxWidth().padding(40.dp), contentAlignment = Alignment.Center) {
        Text(text, fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.muted, textAlign = TextAlign.Center)
    }
}

@Composable
private fun HeroCard(kicker: String, hero: String, sub: String, accent: Boolean = false) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier.fillMaxSize().padding(28.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(kicker.uppercase(), fontSize = 13.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, letterSpacing = 1.sp)
        Spacer(Modifier.height(12.dp))
        Text(hero, fontSize = 56.sp, fontWeight = FontWeight.Black, fontFamily = FontFamily.Monospace,
            color = if (accent) colors.accent else colors.text, textAlign = TextAlign.Center)
        Spacer(Modifier.height(12.dp))
        Text(sub, fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.muted, textAlign = TextAlign.Center)
    }
}

@Composable
private fun CardScaffold(title: String, content: @Composable ColumnScope.() -> Unit) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier.fillMaxSize().padding(horizontal = 28.dp).verticalScroll(rememberScrollState()),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(title, fontSize = 22.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, color = colors.text)
        Spacer(Modifier.height(20.dp))
        content()
    }
}

@Composable
private fun RankRow(rank: Int, coverUrl: String?, name: String, meta: String?, count: Int) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("$rank", fontSize = 18.sp, fontWeight = FontWeight.Black, fontFamily = FontFamily.Monospace,
            color = colors.accent, modifier = Modifier.width(28.dp))
        if (coverUrl != null || meta != null) {
            CoverImage(url = coverUrl, contentDescription = null,
                modifier = Modifier.size(40.dp).clip(RoundedCornerShape(6.dp)))
            Spacer(Modifier.width(10.dp))
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(name, fontSize = 14.sp, fontWeight = FontWeight.SemiBold, fontFamily = FontFamily.Monospace,
                color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (meta != null) {
                Text(meta, fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Spacer(Modifier.width(8.dp))
        Text("$count", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
    }
}

@Composable
private fun TrackListCard(title: String, tracks: List<com.fossisawesome.firmium.data.db.TrackStat>, coverUrlFor: (String?) -> String?) {
    CardScaffold(title) {
        tracks.take(5).forEachIndexed { i, t ->
            RankRow(i + 1, coverUrlFor(t.coverArtId), t.title, t.artist, t.count)
        }
    }
}

@Composable
private fun ArtistListCard(title: String, artists: List<com.fossisawesome.firmium.data.db.ArtistStat>) {
    CardScaffold(title) {
        artists.take(5).forEachIndexed { i, a ->
            RankRow(i + 1, null, a.name, null, a.count)
        }
    }
}

@Composable
private fun AlbumListCard(title: String, albums: List<com.fossisawesome.firmium.data.db.AlbumStat>, coverUrlFor: (String?) -> String?) {
    CardScaffold(title) {
        albums.take(5).forEachIndexed { i, a ->
            RankRow(i + 1, coverUrlFor(a.coverArtId), a.name, a.artist, a.count)
        }
    }
}

@Composable
private fun TimeOfDayCard(s: RecapStats) {
    val colors = LocalFirmiumColors.current
    val t = s.timeOfDay
    val rows = listOf("Morning" to t.morning, "Afternoon" to t.afternoon, "Evening" to t.evening, "Night" to t.night)
    val max = (rows.maxOf { it.second }).coerceAtLeast(1)
    CardScaffold("By Time of Day") {
        rows.forEach { (label, value) ->
            Row(modifier = Modifier.fillMaxWidth().padding(vertical = 7.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(label, fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.text, modifier = Modifier.width(84.dp))
                Box(modifier = Modifier.weight(1f).height(12.dp).clip(RoundedCornerShape(50)).background(colors.surface2)) {
                    Box(modifier = Modifier.fillMaxHeight().fillMaxWidth(value.toFloat() / max).clip(RoundedCornerShape(50)).background(colors.accent))
                }
                Spacer(Modifier.width(8.dp))
                Text("$value", fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted, modifier = Modifier.width(36.dp))
            }
        }
    }
}

@Composable
private fun DayOfWeekCard(s: RecapStats) {
    val colors = LocalFirmiumColors.current
    val max = (s.byDayOfWeek.maxOrNull() ?: 1).coerceAtLeast(1)
    CardScaffold("By Day of Week") {
        Row(
            modifier = Modifier.fillMaxWidth().height(200.dp),
            verticalAlignment = Alignment.Bottom,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            s.byDayOfWeek.forEachIndexed { i, value ->
                Column(modifier = Modifier.weight(1f).fillMaxHeight(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Bottom) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .fillMaxHeight((value.toFloat() / max).coerceAtLeast(0.02f))
                            .clip(RoundedCornerShape(topStart = 6.dp, topEnd = 6.dp))
                            .background(colors.accent),
                    )
                    Spacer(Modifier.height(6.dp))
                    Text(DOW[i], fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                }
            }
        }
    }
}

@Composable
private fun DiscoveryCard(d: DiscoveryStat?, coverUrlFor: (String?) -> String?) {
    val colors = LocalFirmiumColors.current
    CardScaffold("Biggest Discovery") {
        if (d == null) {
            Text("Not enough plays yet", fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        } else {
            CoverImage(url = coverUrlFor(d.coverArtId), contentDescription = null,
                modifier = Modifier.size(160.dp).clip(RoundedCornerShape(12.dp)))
            Spacer(Modifier.height(16.dp))
            Text(d.title, fontSize = 20.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace,
                color = colors.text, textAlign = TextAlign.Center, maxLines = 2, overflow = TextOverflow.Ellipsis)
            d.artist?.let {
                Text(it, fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
            }
            Spacer(Modifier.height(8.dp))
            val date = SimpleDateFormat("MMM d, yyyy", Locale.getDefault()).format(Date(d.firstHeard * 1000))
            Text("${d.count} plays · first heard $date", fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                color = colors.muted, textAlign = TextAlign.Center)
        }
    }
}

@Composable
private fun SummaryCard(s: RecapStats) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Brush.verticalGradient(listOf(colors.bg, colors.surface)))
            .padding(28.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.Center,
    ) {
        Text("FIRMIUM RECAP", fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.accent, letterSpacing = 2.sp)
        Spacer(Modifier.height(8.dp))
        Text(formatDuration(s.totalSeconds), fontSize = 48.sp, fontWeight = FontWeight.Black,
            fontFamily = FontFamily.Monospace, color = colors.text)
        Text("${s.totalPlays} tracks · ${s.daysActive} active days", fontSize = 14.sp,
            fontFamily = FontFamily.Monospace, color = colors.muted)
        Spacer(Modifier.height(20.dp))
        s.topTracks.firstOrNull()?.let { SummaryLine("Top track", it.title) }
        s.topArtists.firstOrNull()?.let { SummaryLine("Top artist", it.name) }
        s.topAlbums.firstOrNull()?.let { SummaryLine("Top album", it.name) }
        s.topGenre?.let { SummaryLine("Top genre", it.genre) }
    }
}

@Composable
private fun SummaryLine(label: String, value: String) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.padding(vertical = 6.dp)) {
        Text(label.uppercase(), fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted, letterSpacing = 0.5.sp)
        Text(value, fontSize = 18.sp, fontWeight = FontWeight.SemiBold, fontFamily = FontFamily.Monospace,
            color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}
