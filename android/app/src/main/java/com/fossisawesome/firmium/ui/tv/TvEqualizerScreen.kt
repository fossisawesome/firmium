package com.fossisawesome.firmium.ui.tv

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.eq.EqBand
import com.fossisawesome.firmium.data.eq.EqProfile
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.ui.components.FirmiumTextField
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.launch

private val GRAPHIC_FREQS = listOf(31f, 62f, 125f, 250f, 500f, 1000f, 2000f, 4000f, 8000f, 16000f)
private val EQ_PROFILES_TYPE = object : TypeToken<List<EqProfile>>() {}.type

private fun freqLabel(f: Float): String = if (f >= 1000f) "${(f / 1000f).toInt()}k" else "${f.toInt()}"

// Graphic-mode band editing only (D-pad +/- steppers) — parametric editing, .toml import,
// and profile deletion are phone/desktop-only for now (text-heavy, awkward on a remote).
@Composable
fun TvEqualizerScreen(onBack: () -> Unit) {
    val colors = LocalFirmiumColors.current
    val context = LocalContext.current
    val prefs = remember { AppPreferences(context) }
    val scope = rememberCoroutineScope()
    val gson = remember { Gson() }

    BackHandler { onBack() }

    val enabled by prefs.eqEnabled.collectAsState(initial = false)
    val activeName by prefs.eqActiveProfile.collectAsState(initial = null)
    val profilesJson by prefs.eqProfilesJson.collectAsState(initial = null)

    val profiles: List<EqProfile> = remember(profilesJson) {
        profilesJson?.let { runCatching { gson.fromJson<List<EqProfile>>(it, EQ_PROFILES_TYPE) }.getOrNull() }.orEmpty()
    }
    val active = profiles.firstOrNull { it.name == activeName } ?: profiles.firstOrNull()

    fun persist(updated: List<EqProfile>, activate: String? = null) {
        scope.launch {
            prefs.setEqProfilesJson(gson.toJson(updated))
            if (activate != null) prefs.setEqActiveProfile(activate)
        }
    }

    var newName by remember { mutableStateOf("") }

    LazyColumn(modifier = Modifier.fillMaxSize().padding(48.dp), content = {
        item {
            Text(text = "Equalizer", color = colors.text, fontSize = 24.sp, modifier = Modifier.padding(bottom = 24.dp))

            TvToggleRow(
                label = "Enable Equalizer",
                checked = enabled,
                colors = colors,
                onToggle = { v -> scope.launch { prefs.setEqEnabled(v) } },
                modifier = Modifier.padding(bottom = 24.dp),
            )

            Text(text = "Profiles", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 8.dp))
            if (profiles.isEmpty()) {
                Text(text = "No profiles yet — create one below.", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 16.dp))
            } else {
                profiles.forEach { profile ->
                    val selected = profile.name == active?.name
                    TvActionButton(
                        onClick = { scope.launch { prefs.setEqActiveProfile(profile.name) } },
                        colors = colors,
                        modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
                    ) {
                        Text(text = "${profile.name} (${profile.mode})", color = if (selected) colors.accent else colors.text, fontSize = 14.sp)
                    }
                }
            }

            if (active != null && active.mode == "graphic") {
                Text(text = "Bands — ${active.name}", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(top = 16.dp, bottom = 8.dp))
                active.bands.forEachIndexed { index, band ->
                    TvStepperRow(
                        label = "${freqLabel(band.freq)} Hz",
                        valueText = "${if (band.gain > 0) "+" else ""}${band.gain.toInt()} dB",
                        colors = colors,
                        onDecrement = {
                            val updatedBands = active.bands.toMutableList()
                            updatedBands[index] = band.copy(gain = (band.gain - 1f).coerceIn(-12f, 12f))
                            persist(profiles.map { if (it.name == active.name) it.copy(bands = updatedBands) else it })
                        },
                        onIncrement = {
                            val updatedBands = active.bands.toMutableList()
                            updatedBands[index] = band.copy(gain = (band.gain + 1f).coerceIn(-12f, 12f))
                            persist(profiles.map { if (it.name == active.name) it.copy(bands = updatedBands) else it })
                        },
                    )
                }
            }

            Text(text = "New profile", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(top = 24.dp, bottom = 8.dp))
            FirmiumTextField(
                value = newName,
                onValueChange = { newName = it },
                placeholder = "Profile name",
                modifier = Modifier.width(360.dp).padding(bottom = 12.dp),
            )
            TvActionButton(
                onClick = {
                    if (newName.isNotBlank()) {
                        val profile = EqProfile(newName, "graphic", GRAPHIC_FREQS.map { EqBand(it, 0f) })
                        persist(profiles + profile, activate = profile.name)
                        newName = ""
                    }
                },
                colors = colors,
            ) {
                Text(text = "Create Graphic Profile", color = colors.text, fontSize = 14.sp)
            }
        }
    })
}
