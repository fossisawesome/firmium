package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.RadioSeeder
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

private data class EnergyOption(val energy: RadioSeeder.Energy, val label: String, val desc: String)

private val ENERGIES = listOf(
    EnergyOption(RadioSeeder.Energy.CHILL, "Chill", "Under 80 BPM"),
    EnergyOption(RadioSeeder.Energy.MID, "Mid", "80–120 BPM"),
    EnergyOption(RadioSeeder.Energy.HIGH, "High", "120+ BPM"),
)

// Mood Mix — pick an energy band (+ optional genre) and play a shuffled queue.
@Composable
fun MixScreen(
    genres: List<String>,
    onStartMix: (RadioSeeder.Energy, String?) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var energy by remember { mutableStateOf(RadioSeeder.Energy.MID) }
    var genre by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(20.dp),
    ) {
        Text("Mix", fontSize = 26.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current, color = colors.text)
        Spacer(Modifier.height(4.dp))
        Text("Generate a shuffled queue tuned to an energy level.",
            fontSize = 13.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)

        Spacer(Modifier.height(24.dp))
        Text("ENERGY", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted, letterSpacing = 1.sp)
        Spacer(Modifier.height(10.dp))
        ENERGIES.forEach { opt ->
            val selected = energy == opt.energy
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 4.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .border(1.dp, if (selected) colors.accent else colors.border, RoundedCornerShape(8.dp))
                    .background(if (selected) colors.surface2 else colors.surface)
                    .clickable { energy = opt.energy }
                    .padding(horizontal = 16.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(opt.label, fontSize = 15.sp, fontWeight = FontWeight.SemiBold, fontFamily = LocalAppFontFamily.current, color = colors.text)
                    Text(opt.desc, fontSize = 12.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
                }
            }
        }

        if (genres.isNotEmpty()) {
            Spacer(Modifier.height(24.dp))
            Text("GENRE (OPTIONAL)", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted, letterSpacing = 1.sp)
            Spacer(Modifier.height(10.dp))
            FlowGenres(genres = genres, selected = genre, onSelect = { genre = if (genre == it) null else it })
        }

        Spacer(Modifier.height(28.dp))
        Box(
            modifier = Modifier
                .clip(RoundedCornerShape(999.dp))
                .background(colors.accent)
                .clickable { onStartMix(energy, genre) }
                .padding(horizontal = 28.dp, vertical = 14.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text("Start Mix", fontSize = 15.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
                color = androidx.compose.ui.graphics.Color.Black)
        }
    }
}

// Simple wrapping chip row for genre selection.
@Composable
private fun FlowGenres(genres: List<String>, selected: String?, onSelect: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    Column {
        genres.chunked(2).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { g ->
                    val sel = selected == g
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .padding(vertical = 4.dp)
                            .clip(RoundedCornerShape(6.dp))
                            .border(1.dp, if (sel) colors.accent else colors.border, RoundedCornerShape(6.dp))
                            .background(if (sel) colors.surface2 else colors.surface)
                            .clickable { onSelect(g) }
                            .padding(horizontal = 12.dp, vertical = 10.dp),
                    ) {
                        Text(g, fontSize = 13.sp, fontFamily = LocalAppFontFamily.current,
                            color = if (sel) colors.accent else colors.text, maxLines = 1)
                    }
                }
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
        }
    }
}
