package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.audio.PlayerState
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.ALL_THEMES
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

private val fontOptions = listOf(
    "Liberation Mono", "Inter", "Monospace", "Sans Serif", "Cousine", "FiraCode", "Hack", "BigBlue Terminal",
)
private val visualizerTypeOptions = listOf("orb", "bars", "oscilloscope")
private val crossfadeCurveOptions = listOf("linear", "logarithmic")

// Covers the core playback/appearance settings that matter on TV. Last.fm key/secret entry,
// cache wipe/clear, and reset-settings are left off (use the phone/desktop app) — typing
// secrets and destructive maintenance actions aren't a good fit for a remote-driven form.
@Composable
fun TvSettingsScreen(
    playerState: PlayerState,
    serverUrl: String,
    username: String,
    appVersion: String,
    currentThemeId: String,
    currentFontFamily: String,
    onThemeSelected: (String) -> Unit,
    onFontSelected: (String) -> Unit,
    onCrossfadeToggle: (Boolean) -> Unit,
    onCrossfadeDurationChange: (Int) -> Unit,
    onCrossfadeCurveChange: (String) -> Unit,
    onGaplessToggle: (Boolean) -> Unit,
    onReplayGainToggle: (Boolean) -> Unit,
    onVisualizerToggle: (Boolean) -> Unit,
    onVisualizerTypeSelected: (String) -> Unit,
    onLogout: () -> Unit,
    onOpenEqualizer: () -> Unit,
    onViewRecap: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val themeIndex = ALL_THEMES.indexOfFirst { it.id == currentThemeId }.coerceAtLeast(0)
    val fontIndex = fontOptions.indexOf(currentFontFamily).coerceAtLeast(0)
    val crossfadeCurveIndex = crossfadeCurveOptions.indexOf(playerState.crossfadeCurve).coerceAtLeast(0)
    val visualizerTypeIndex = visualizerTypeOptions.indexOf(playerState.visualizerType).coerceAtLeast(0)
    val crossfadeSeconds = playerState.crossfadeDurationMs / 1000

    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(48.dp),
        content = {
            item {
                Text(text = "Settings", color = colors.text, fontSize = 24.sp, modifier = Modifier.padding(bottom = 8.dp))
                Text(text = "Signed in as $username · $serverUrl", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 4.dp))
                Text(text = "Firmium $appVersion", color = colors.muted, fontSize = 12.sp, modifier = Modifier.padding(bottom = 24.dp))

                Text(text = "Appearance", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 8.dp))
                TvCycleRow(
                    label = "Theme",
                    options = ALL_THEMES.map { it.name },
                    selectedIndex = themeIndex,
                    colors = colors,
                    onSelect = { onThemeSelected(ALL_THEMES[it].id) },
                    modifier = Modifier.padding(bottom = 8.dp),
                )
                TvCycleRow(
                    label = "Font",
                    options = fontOptions,
                    selectedIndex = fontIndex,
                    colors = colors,
                    onSelect = { onFontSelected(fontOptions[it]) },
                    modifier = Modifier.padding(bottom = 24.dp),
                )

                Text(text = "Playback", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 8.dp))
                TvToggleRow(label = "Crossfade", checked = playerState.crossfadeEnabled, colors = colors, onToggle = onCrossfadeToggle, modifier = Modifier.padding(bottom = 8.dp))
                if (playerState.crossfadeEnabled) {
                    TvStepperRow(
                        label = "Crossfade duration",
                        valueText = "${crossfadeSeconds}s",
                        colors = colors,
                        onDecrement = { onCrossfadeDurationChange((crossfadeSeconds - 1).coerceIn(1, 12) * 1000) },
                        onIncrement = { onCrossfadeDurationChange((crossfadeSeconds + 1).coerceIn(1, 12) * 1000) },
                        modifier = Modifier.padding(bottom = 8.dp),
                    )
                    TvCycleRow(
                        label = "Crossfade curve",
                        options = crossfadeCurveOptions,
                        selectedIndex = crossfadeCurveIndex,
                        colors = colors,
                        onSelect = { onCrossfadeCurveChange(crossfadeCurveOptions[it]) },
                        modifier = Modifier.padding(bottom = 8.dp),
                    )
                }
                TvToggleRow(label = "Gapless playback", checked = playerState.gaplessEnabled, colors = colors, onToggle = onGaplessToggle, modifier = Modifier.padding(bottom = 8.dp))
                TvToggleRow(label = "ReplayGain", checked = playerState.replayGainEnabled, colors = colors, onToggle = onReplayGainToggle, modifier = Modifier.padding(bottom = 24.dp))

                Text(text = "Visualizer", color = colors.muted, fontSize = 13.sp, modifier = Modifier.padding(bottom = 8.dp))
                TvToggleRow(label = "Show visualizer", checked = playerState.visualizerEnabled, colors = colors, onToggle = onVisualizerToggle, modifier = Modifier.padding(bottom = 8.dp))
                if (playerState.visualizerEnabled) {
                    TvCycleRow(
                        label = "Visualizer type",
                        options = visualizerTypeOptions,
                        selectedIndex = visualizerTypeIndex,
                        colors = colors,
                        onSelect = { onVisualizerTypeSelected(visualizerTypeOptions[it]) },
                        modifier = Modifier.padding(bottom = 24.dp),
                    )
                }

                TvActionButton(onClick = onOpenEqualizer, colors = colors, modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp)) {
                    Text(text = "Equalizer", color = colors.text, fontSize = 14.sp)
                }
                TvActionButton(onClick = onViewRecap, colors = colors, modifier = Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
                    Text(text = "Recap & Listening Stats", color = colors.text, fontSize = 14.sp)
                }

                TvActionButton(onClick = onLogout, colors = colors, modifier = Modifier.fillMaxWidth()) {
                    Text(text = "Log Out", color = colors.error, fontSize = 14.sp)
                }
            }
        },
    )
}
