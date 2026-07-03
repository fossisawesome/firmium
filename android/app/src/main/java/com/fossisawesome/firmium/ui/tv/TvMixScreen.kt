package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.RadioSeeder
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

private val energyOptions = listOf(
    RadioSeeder.Energy.CHILL to "Chill (under 80 BPM)",
    RadioSeeder.Energy.MID to "Mid (80-120 BPM)",
    RadioSeeder.Energy.HIGH to "High (over 120 BPM)",
)

@Composable
fun TvMixScreen(
    genres: List<String>,
    onStartMix: (RadioSeeder.Energy, String?) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var selectedEnergy by remember { mutableStateOf(RadioSeeder.Energy.MID) }
    var selectedGenre by remember { mutableStateOf<String?>(null) }

    Column(modifier = Modifier.fillMaxSize().padding(48.dp)) {
        Text(text = "Smart Mix", color = colors.text, fontSize = 24.sp, modifier = Modifier.padding(bottom = 24.dp))

        Text(text = "Energy", color = colors.muted, fontSize = 14.sp, modifier = Modifier.padding(bottom = 8.dp))
        Column(modifier = Modifier.padding(bottom = 24.dp)) {
            energyOptions.forEach { (energy, label) ->
                val selected = selectedEnergy == energy
                TvActionButton(onClick = { selectedEnergy = energy }, colors = colors, modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp)) {
                    Text(text = label, color = if (selected) colors.accent else colors.text, fontSize = 14.sp)
                }
            }
        }

        if (genres.isNotEmpty()) {
            Text(text = "Genre (optional)", color = colors.muted, fontSize = 14.sp, modifier = Modifier.padding(bottom = 8.dp))
            LazyRow(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                contentPadding = PaddingValues(bottom = 24.dp),
            ) {
                item {
                    TvActionButton(onClick = { selectedGenre = null }, colors = colors) {
                        Text(text = "Any", color = if (selectedGenre == null) colors.accent else colors.text, fontSize = 13.sp)
                    }
                }
                items(genres.size) { index ->
                    val genre = genres[index]
                    TvActionButton(onClick = { selectedGenre = genre }, colors = colors) {
                        Text(text = genre, color = if (selectedGenre == genre) colors.accent else colors.text, fontSize = 13.sp)
                    }
                }
            }
        }

        TvActionButton(onClick = { onStartMix(selectedEnergy, selectedGenre) }, colors = colors) {
            Text(text = "Start Mix", color = colors.text, fontSize = 15.sp)
        }
    }
}
