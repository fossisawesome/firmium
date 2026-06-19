package com.fossisawesome.firmium.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.NavigateNext
import androidx.compose.material.icons.filled.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.audio.PlayerState
import com.fossisawesome.firmium.data.eq.EqBand
import com.fossisawesome.firmium.data.eq.EqProfile
import com.fossisawesome.firmium.data.eq.TomlEqParser
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.data.storage.SecureStorage
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.ALL_THEMES
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

// Settings screen — category list → sub-panel drill-down, exact port of MobileSettings.svelte.
@Composable
fun SettingsScreen(
    playerState: PlayerState,
    serverUrl: String,
    username: String,
    appVersion: String,
    currentThemeId: String,
    lrclibEnabled: Boolean,
    lyricsWordFillEnabled: Boolean,
    lastfmEnabled: Boolean,
    lastfmApiKey: String,
    lastfmSecret: String,
    autoLoginEnabled: Boolean,
    downloadFormat: String,
    onCrossfadeToggle: (Boolean) -> Unit,
    onCrossfadeDurationChange: (Int) -> Unit,
    onGaplessToggle: (Boolean) -> Unit,
    onReplayGainToggle: (Boolean) -> Unit,
    onThemeSelected: (String) -> Unit,
    onVisualizerToggle: (Boolean) -> Unit,
    onVisualizerTypeSelected: (String) -> Unit,
    onLrclibToggle: (Boolean) -> Unit,
    onLyricsWordFillToggle: (Boolean) -> Unit,
    onLastfmToggle: (Boolean) -> Unit,
    onLastfmApiKeyChange: (String) -> Unit,
    onLastfmSecretChange: (String) -> Unit,
    onAutoLoginToggle: (Boolean) -> Unit,
    onDownloadFormatSelected: (String) -> Unit,
    onWipeCache: () -> Unit,
    onClearCache: () -> Unit,
    onResetSettings: () -> Unit,
    onLogout: () -> Unit,
) {
    FirmiumSettingsScreen(
        playerState = playerState,
        serverUrl = serverUrl,
        username = username,
        appVersion = appVersion,
        currentThemeId = currentThemeId,
        lrclibEnabled = lrclibEnabled,
        lyricsWordFillEnabled = lyricsWordFillEnabled,
        lastfmEnabled = lastfmEnabled,
        lastfmApiKey = lastfmApiKey,
        lastfmSecret = lastfmSecret,
        autoLoginEnabled = autoLoginEnabled,
        downloadFormat = downloadFormat,
        onCrossfadeToggle = onCrossfadeToggle,
        onCrossfadeDurationChange = onCrossfadeDurationChange,
        onGaplessToggle = onGaplessToggle,
        onReplayGainToggle = onReplayGainToggle,
        onThemeSelected = onThemeSelected,
        onVisualizerToggle = onVisualizerToggle,
        onVisualizerTypeSelected = onVisualizerTypeSelected,
        onLrclibToggle = onLrclibToggle,
        onLyricsWordFillToggle = onLyricsWordFillToggle,
        onLastfmToggle = onLastfmToggle,
        onLastfmApiKeyChange = onLastfmApiKeyChange,
        onLastfmSecretChange = onLastfmSecretChange,
        onAutoLoginToggle = onAutoLoginToggle,
        onDownloadFormatSelected = onDownloadFormatSelected,
        onWipeCache = onWipeCache,
        onClearCache = onClearCache,
        onResetSettings = onResetSettings,
        onLogout = onLogout,
    )
}

// ── Firmium variant ───────────────────────────────────────────────────────────
// Exact port of MobileSettings.svelte: category list → sub-panel drill-down.

private data class Category(val id: String, val label: String, val icon: ImageVector)

private val CATEGORIES = listOf(
    Category("appearance", "Appearance", Icons.Default.Palette),
    Category("playback",   "Playback",   Icons.Default.PlayArrow),
    Category("equalizer",  "Equalizer",  Icons.Default.GraphicEq),
    Category("downloads",  "Downloads",  Icons.Default.Download),
    Category("services",   "Services",   Icons.Default.Language),
    Category("account",    "Account",    Icons.Default.Person),
    Category("about",      "About",      Icons.Default.Info),
)

private data class DownloadFormatOption(val id: String, val name: String)

private val FORMAT_OPTIONS = listOf(
    DownloadFormatOption("original", "Original"),
    DownloadFormatOption("mp3",      "MP3"),
    DownloadFormatOption("flac",     "FLAC"),
    DownloadFormatOption("wav",      "WAV"),
    DownloadFormatOption("opus",     "Opus"),
)

@Composable
private fun FirmiumSettingsScreen(
    playerState: PlayerState,
    serverUrl: String,
    username: String,
    appVersion: String,
    currentThemeId: String,
    lrclibEnabled: Boolean,
    lyricsWordFillEnabled: Boolean,
    lastfmEnabled: Boolean,
    lastfmApiKey: String,
    lastfmSecret: String,
    autoLoginEnabled: Boolean,
    downloadFormat: String,
    onCrossfadeToggle: (Boolean) -> Unit,
    onCrossfadeDurationChange: (Int) -> Unit,
    onGaplessToggle: (Boolean) -> Unit,
    onReplayGainToggle: (Boolean) -> Unit,
    onThemeSelected: (String) -> Unit,
    onVisualizerToggle: (Boolean) -> Unit,
    onVisualizerTypeSelected: (String) -> Unit,
    onLrclibToggle: (Boolean) -> Unit,
    onLyricsWordFillToggle: (Boolean) -> Unit,
    onLastfmToggle: (Boolean) -> Unit,
    onLastfmApiKeyChange: (String) -> Unit,
    onLastfmSecretChange: (String) -> Unit,
    onAutoLoginToggle: (Boolean) -> Unit,
    onDownloadFormatSelected: (String) -> Unit,
    onWipeCache: () -> Unit,
    onClearCache: () -> Unit,
    onResetSettings: () -> Unit,
    onLogout: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val border = colors.border

    // null = category list; non-null = open sub-panel id
    var activeCategory by remember { mutableStateOf<String?>(null) }

    Column(modifier = Modifier.fillMaxSize()) {
        // Header — matches .mset-header: padding 12dp, border-bottom, gap 12dp
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // Back/close button — matches .mset-back-btn (44x44dp circle)
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(50))
                    .clickable { if (activeCategory != null) activeCategory = null },
                contentAlignment = Alignment.Center,
            ) {
                FirmiumIcon(
                    Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Back",
                    tint = if (activeCategory != null) colors.text else Color.Transparent,
                )
            }
            // Title — matches .mset-title: 18sp bold
            Text(
                text = if (activeCategory != null)
                    CATEGORIES.find { it.id == activeCategory }?.label ?: "Settings"
                else "Settings",
                fontSize = 18.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = FontFamily.Monospace,
                color = colors.text,
            )
        }
        FirmiumDivider()

        // Body — slides left when opening a sub-panel, slides right when going back.
        AnimatedContent(
            targetState = activeCategory,
            transitionSpec = {
                val goingDeeper = targetState != null
                val enter = if (goingDeeper) {
                    slideInHorizontally(initialOffsetX = { it }, animationSpec = tween(280)) +
                        fadeIn(tween(220))
                } else {
                    slideInHorizontally(initialOffsetX = { -it / 4 }, animationSpec = tween(280)) +
                        fadeIn(tween(220))
                }
                val exit = if (goingDeeper) {
                    slideOutHorizontally(targetOffsetX = { -it / 4 }, animationSpec = tween(280)) +
                        fadeOut(tween(220))
                } else {
                    slideOutHorizontally(targetOffsetX = { it }, animationSpec = tween(280)) +
                        fadeOut(tween(220))
                }
                enter togetherWith exit
            },
            label = "settingsPanel",
            modifier = Modifier.fillMaxSize(),
        ) { category ->
            if (category == null) {
                // Category list
                Column(modifier = Modifier.fillMaxWidth()) {
                    CATEGORIES.forEachIndexed { i, cat ->
                        // .mset-cat-row: padding 16/20dp, font-size 15sp, gap 14dp
                        if (i == 0) FirmiumDivider()
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { activeCategory = cat.id }
                                .padding(horizontal = 20.dp, vertical = 16.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(14.dp),
                        ) {
                            // .mset-cat-icon: accent color
                            FirmiumIcon(cat.icon, contentDescription = null,
                                tint = colors.accent, modifier = Modifier.size(20.dp))
                            // .mset-cat-label: 15sp text color
                            Text(cat.label, fontSize = 15.sp, fontFamily = FontFamily.Monospace,
                                color = colors.text, modifier = Modifier.weight(1f))
                            // .mset-cat-chevron: muted color
                            FirmiumIcon(Icons.AutoMirrored.Filled.NavigateNext, contentDescription = null,
                                tint = colors.muted, modifier = Modifier.size(16.dp))
                        }
                        FirmiumDivider()
                    }
                }
            } else {
                // Sub-panel — matches .mset-subpanel--in
                val scrollState = rememberScrollState()
                Box(modifier = Modifier.fillMaxSize()) {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .verticalScroll(scrollState),
                    ) {
                    when (category) {
                        "appearance" -> FirmiumAppearancePanel(
                            currentThemeId = currentThemeId,
                            onThemeSelected = onThemeSelected,
                            visualizerEnabled = playerState.visualizerEnabled,
                            visualizerType = playerState.visualizerType,
                            onVisualizerToggle = onVisualizerToggle,
                            onVisualizerTypeSelected = onVisualizerTypeSelected,
                        )
                        "playback" -> FirmiumPlaybackPanel(
                            playerState = playerState,
                            onCrossfadeToggle = onCrossfadeToggle,
                            onCrossfadeDurationChange = onCrossfadeDurationChange,
                            onGaplessToggle = onGaplessToggle,
                            onReplayGainToggle = onReplayGainToggle,
                        )
                        "equalizer" -> FirmiumEqualizerPanel()
                        "downloads" -> FirmiumDownloadsPanel(
                            downloadFormat = downloadFormat,
                            onDownloadFormatSelected = onDownloadFormatSelected,
                        )
                        "services" -> FirmiumServicesPanel(
                            lrclibEnabled = lrclibEnabled,
                            lyricsWordFillEnabled = lyricsWordFillEnabled,
                            lastfmEnabled = lastfmEnabled,
                            lastfmApiKey = lastfmApiKey,
                            lastfmSecret = lastfmSecret,
                            onLrclibToggle = onLrclibToggle,
                            onLyricsWordFillToggle = onLyricsWordFillToggle,
                            onLastfmToggle = onLastfmToggle,
                            onLastfmApiKeyChange = onLastfmApiKeyChange,
                            onLastfmSecretChange = onLastfmSecretChange,
                        )
                        "account" -> FirmiumAccountPanel(
                            serverUrl = serverUrl,
                            username = username,
                            autoLoginEnabled = autoLoginEnabled,
                            onAutoLoginToggle = onAutoLoginToggle,
                            onLogout = onLogout,
                        )
                        "about" -> FirmiumAboutPanel(
                            appVersion = appVersion,
                            onWipeCache = onWipeCache,
                            onClearCache = onClearCache,
                            onResetSettings = onResetSettings,
                        )
                    }
                    }
                    FirmiumVerticalScrollbar(scrollState, modifier = Modifier.align(Alignment.TopEnd))
                }
            }
        }
    }
}

// ── Firmium sub-panels ────────────────────────────────────────────────────────
// Each panel matches the .settings-row style: title + desc on left, control on right.

@Composable
private fun FirmiumSettingsRow(
    title: String,
    desc: String,
    content: @Composable () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(title, fontSize = 15.sp, fontFamily = FontFamily.Monospace, color = colors.text)
            Spacer(Modifier.height(2.dp))
            Text(desc, fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        }
        Spacer(Modifier.width(12.dp))
        content()
    }
    FirmiumDivider()
}


@Composable
private fun FirmiumAppearancePanel(
    currentThemeId: String,
    onThemeSelected: (String) -> Unit,
    visualizerEnabled: Boolean,
    visualizerType: String,
    onVisualizerToggle: (Boolean) -> Unit,
    onVisualizerTypeSelected: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.fillMaxWidth().padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Color Theme", fontSize = 12.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, modifier = Modifier.padding(bottom = 4.dp))
        ThemeDropdown(currentThemeId = currentThemeId, onThemeSelected = onThemeSelected)

        Text("Visualizer", fontSize = 12.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, modifier = Modifier.padding(top = 8.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Text("Audio Visualizer", fontSize = 15.sp, fontFamily = FontFamily.Monospace, color = colors.text)
                Spacer(Modifier.height(2.dp))
                Text("Show an audio-reactive visualizer on the now playing screen",
                    fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
            }
            Spacer(Modifier.width(12.dp))
            FirmiumSwitch(checked = visualizerEnabled, onCheckedChange = onVisualizerToggle)
        }
        if (visualizerEnabled) {
            VisualizerDropdown(visualizerType = visualizerType, onTypeSelected = onVisualizerTypeSelected)
        }
    }
}

@Composable
private fun VisualizerDropdown(visualizerType: String, onTypeSelected: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    val current = VisualizerType.fromId(visualizerType)
    var expanded by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(6.dp))
                .border(1.dp, colors.border, RoundedCornerShape(6.dp))
                .clickable { expanded = !expanded }
                .padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(current.label, fontSize = 14.sp, fontFamily = FontFamily.Monospace,
                color = colors.text, modifier = Modifier.weight(1f))
            FirmiumIcon(
                if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = null, tint = colors.muted, modifier = Modifier.size(18.dp),
            )
        }
        AnimatedVisibility(visible = expanded) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(bottomStart = 6.dp, bottomEnd = 6.dp))
                    .border(1.dp, colors.border, RoundedCornerShape(bottomStart = 6.dp, bottomEnd = 6.dp))
                    .background(colors.surface),
            ) {
                VisualizerType.entries.forEachIndexed { i, t ->
                    if (i > 0) FirmiumDivider(color = colors.border)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onTypeSelected(t.id); expanded = false }
                            .background(if (t.id == visualizerType) colors.surface2.copy(alpha = 0.5f) else Color.Transparent)
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Text(
                            t.label, fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                            color = if (t.id == visualizerType) colors.accent else colors.text,
                            modifier = Modifier.weight(1f),
                        )
                        if (t.id == visualizerType) {
                            FirmiumIcon(Icons.Default.Check, contentDescription = null,
                                tint = colors.accent, modifier = Modifier.size(14.dp))
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FirmiumDownloadsPanel(
    downloadFormat: String,
    onDownloadFormatSelected: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.fillMaxWidth().padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Download Format", fontSize = 12.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, modifier = Modifier.padding(bottom = 4.dp))
        Text(
            "Format used when downloading tracks and albums. \"Original\" saves the file exactly as stored on the server.",
            fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
        )
        FormatDropdown(downloadFormat = downloadFormat, onFormatSelected = onDownloadFormatSelected)
    }
}

@Composable
private fun FormatDropdown(downloadFormat: String, onFormatSelected: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    val current = FORMAT_OPTIONS.find { it.id == downloadFormat } ?: FORMAT_OPTIONS.first()
    var expanded by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(6.dp))
                .border(1.dp, colors.border, RoundedCornerShape(6.dp))
                .clickable { expanded = !expanded }
                .padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(current.name, fontSize = 14.sp, fontFamily = FontFamily.Monospace,
                color = colors.text, modifier = Modifier.weight(1f))
            FirmiumIcon(
                if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = null, tint = colors.muted, modifier = Modifier.size(18.dp),
            )
        }

        AnimatedVisibility(visible = expanded) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(bottomStart = 6.dp, bottomEnd = 6.dp))
                    .border(1.dp, colors.border, RoundedCornerShape(bottomStart = 6.dp, bottomEnd = 6.dp))
                    .background(colors.surface),
            ) {
                FORMAT_OPTIONS.forEachIndexed { i, fmt ->
                    if (i > 0) FirmiumDivider(color = colors.border)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onFormatSelected(fmt.id); expanded = false }
                            .background(if (fmt.id == downloadFormat) colors.surface2.copy(alpha = 0.5f) else Color.Transparent)
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Text(
                            fmt.name, fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                            color = if (fmt.id == downloadFormat) colors.accent else colors.text,
                            modifier = Modifier.weight(1f),
                        )
                        if (fmt.id == downloadFormat) {
                            FirmiumIcon(Icons.Default.Check, contentDescription = null,
                                tint = colors.accent, modifier = Modifier.size(14.dp))
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FirmiumPlaybackPanel(
    playerState: PlayerState,
    onCrossfadeToggle: (Boolean) -> Unit,
    onCrossfadeDurationChange: (Int) -> Unit,
    onGaplessToggle: (Boolean) -> Unit,
    onReplayGainToggle: (Boolean) -> Unit,
) {
    val colors = LocalFirmiumColors.current

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Crossfade", fontSize = 15.sp, fontFamily = FontFamily.Monospace, color = colors.text)
            Spacer(Modifier.height(2.dp))
            Text("Smoothly blend between tracks", fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        }
        Spacer(Modifier.width(12.dp))
        FirmiumSwitch(
            checked = playerState.crossfadeEnabled,
            onCheckedChange = onCrossfadeToggle,
        )
    }
    FirmiumDivider()

    if (playerState.crossfadeEnabled) {
        Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp)) {
            Text(
                "Crossfade: ${playerState.crossfadeDurationMs / 1000}s",
                fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted,
            )
            FirmiumSlider(
                value = playerState.crossfadeDurationMs.toFloat(),
                onValueChange = { onCrossfadeDurationChange(it.toInt()) },
                valueRange = 1000f..12000f,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        FirmiumDivider()
    }

    FirmiumSettingsRow("Gapless Playback", "Pre-buffer the next track for seamless transitions") {
        FirmiumSwitch(checked = playerState.gaplessEnabled, onCheckedChange = onGaplessToggle)
    }

    FirmiumSettingsRow("ReplayGain", "Normalize track loudness using server-provided gain values") {
        FirmiumSwitch(checked = playerState.replayGainEnabled, onCheckedChange = onReplayGainToggle)
    }

    FirmiumAutoContinueRow()
}

// Self-contained Smart Radio toggle — reads/writes the auto-continue pref directly
// so it doesn't need threading through the SettingsScreen signature.
@Composable
private fun FirmiumAutoContinueRow() {
    val context = LocalContext.current
    val prefs = remember { AppPreferences(context) }
    val scope = rememberCoroutineScope()
    val enabled by prefs.autoContinueEnabled.collectAsState(initial = false)
    FirmiumSettingsRow("Continue playing after queue ends",
        "Smart Radio adds similar tracks when the queue runs out") {
        FirmiumSwitch(checked = enabled, onCheckedChange = { v -> scope.launch { prefs.setAutoContinueEnabled(v) } })
    }
}

private val GRAPHIC_FREQS = listOf(31f, 62f, 125f, 250f, 500f, 1000f, 2000f, 4000f, 8000f, 16000f)
private val EQ_PROFILES_TYPE = object : TypeToken<List<EqProfile>>() {}.type

private fun freqLabel(f: Float): String = if (f >= 1000f) "${(f / 1000f).toInt()}k" else "${f.toInt()}"

@Composable
private fun FirmiumEqualizerPanel() {
    val colors = LocalFirmiumColors.current
    val context = LocalContext.current
    val prefs = remember { AppPreferences(context) }
    val scope = rememberCoroutineScope()
    val gson = remember { Gson() }

    val enabled by prefs.eqEnabled.collectAsState(initial = false)
    val activeName by prefs.eqActiveProfile.collectAsState(initial = null)
    val profilesJson by prefs.eqProfilesJson.collectAsState(initial = null)

    val profiles: List<EqProfile> = remember(profilesJson) {
        profilesJson?.let { runCatching { gson.fromJson<List<EqProfile>>(it, EQ_PROFILES_TYPE) }.getOrNull() }.orEmpty()
    }
    val active = profiles.firstOrNull { it.name == activeName } ?: profiles.firstOrNull()

    var newName by remember { mutableStateOf("") }

    fun persist(updated: List<EqProfile>, activate: String?) {
        scope.launch {
            prefs.setEqProfilesJson(gson.toJson(updated))
            if (activate != null) prefs.setEqActiveProfile(activate)
        }
    }

    fun updateActiveBands(bands: List<EqBand>) {
        val a = active ?: return
        persist(profiles.map { if (it.name == a.name) it.copy(bands = bands) else it }, a.name)
    }

    val importLauncher = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch(Dispatchers.IO) {
            val text = runCatching {
                context.contentResolver.openInputStream(uri)?.bufferedReader()?.use { it.readText() }
            }.getOrNull() ?: return@launch
            val imported = TomlEqParser.parse(text)
            if (imported.isEmpty()) return@launch
            val merged = profiles.filter { p -> imported.none { it.name == p.name } } + imported
            prefs.setEqProfilesJson(gson.toJson(merged))
            prefs.setEqActiveProfile(imported.first().name)
        }
    }

    FirmiumSettingsRow("Enable Equalizer", "Apply the active profile to playback") {
        FirmiumSwitch(checked = enabled, onCheckedChange = { v -> scope.launch { prefs.setEqEnabled(v) } })
    }

    // Profile picker
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp)) {
        Text("Profiles", fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        Spacer(Modifier.height(6.dp))
        if (profiles.isEmpty()) {
            Text("No saved profiles. Import a .toml or save one below.",
                fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        } else {
            profiles.forEach { p ->
                Row(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically) {
                    Box(modifier = Modifier.weight(1f).clickable { persist(profiles, p.name) }) {
                        Text(
                            (if (active?.name == p.name) "● " else "○ ") + "${p.name} (${p.mode})",
                            fontSize = 14.sp, fontFamily = FontFamily.Monospace,
                            color = if (active?.name == p.name) colors.text else colors.muted,
                        )
                    }
                    FirmiumTextButton(onClick = {
                        val remaining = profiles.filter { it.name != p.name }
                        persist(remaining, remaining.firstOrNull()?.name)
                    }) {
                        Text("Delete", fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                    }
                }
            }
        }
    }
    FirmiumDivider()

    FirmiumTextButton(
        onClick = { importLauncher.launch(arrayOf("*/*")) },
        modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
    ) {
        Text("Import profile (.toml)", fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.text)
    }
    FirmiumDivider()

    // Editor for the active profile
    val a = active
    if (a != null) {
        if (a.mode == "graphic") {
            Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp)) {
                a.bands.forEachIndexed { i, band ->
                    Text("${freqLabel(band.freq)} Hz: ${if (band.gain > 0) "+" else ""}${band.gain.toInt()} dB",
                        fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                    FirmiumSlider(
                        value = band.gain,
                        onValueChange = { g ->
                            updateActiveBands(a.bands.toMutableList().also { it[i] = band.copy(gain = g) })
                        },
                        valueRange = -12f..12f,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
        } else {
            Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)) {
                a.bands.forEachIndexed { i, band ->
                    Row(verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        val numberKeyboard = KeyboardOptions(keyboardType = KeyboardType.Number)
                        FirmiumTextField(value = band.freq.toInt().toString(),
                            onValueChange = { v -> v.toFloatOrNull()?.let { updateActiveBands(a.bands.toMutableList().also { l -> l[i] = band.copy(freq = it) }) } },
                            label = "Hz", keyboardOptions = numberKeyboard, modifier = Modifier.weight(1f))
                        FirmiumTextField(value = band.gain.toString(),
                            onValueChange = { v -> v.toFloatOrNull()?.let { updateActiveBands(a.bands.toMutableList().also { l -> l[i] = band.copy(gain = it) }) } },
                            label = "dB", keyboardOptions = numberKeyboard, modifier = Modifier.weight(1f))
                        FirmiumTextField(value = (band.q ?: 1.0f).toString(),
                            onValueChange = { v -> v.toFloatOrNull()?.let { updateActiveBands(a.bands.toMutableList().also { l -> l[i] = band.copy(q = it) }) } },
                            label = "Q", keyboardOptions = numberKeyboard, modifier = Modifier.weight(1f))
                        FirmiumTextButton(onClick = { updateActiveBands(a.bands.filterIndexed { idx, _ -> idx != i }) }) {
                            Text("×", fontSize = 16.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                        }
                    }
                }
                FirmiumTextButton(onClick = { updateActiveBands(a.bands + EqBand(1000f, 0f, 1.0f)) }) {
                    Text("Add band", fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.text)
                }
            }
        }
        FirmiumDivider()
    }

    // Save as a new profile (graphic or parametric)
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)) {
        FirmiumTextField(value = newName, onValueChange = { newName = it }, label = "New profile name",
            modifier = Modifier.fillMaxWidth())
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FirmiumTextButton(onClick = {
                val name = newName.trim()
                if (name.isEmpty()) return@FirmiumTextButton
                val profile = EqProfile(name, "graphic", GRAPHIC_FREQS.map { EqBand(it, 0f) })
                persist(profiles.filter { it.name != name } + profile, name)
                newName = ""
            }) { Text("Save Graphic", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.text) }
            FirmiumTextButton(onClick = {
                val name = newName.trim()
                if (name.isEmpty()) return@FirmiumTextButton
                val profile = EqProfile(name, "parametric", listOf(EqBand(100f, 0f, 1f), EqBand(1000f, 0f, 1f), EqBand(8000f, 0f, 1f)))
                persist(profiles.filter { it.name != name } + profile, name)
                newName = ""
            }) { Text("Save Parametric", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.text) }
        }
    }
}

@Composable
private fun FirmiumServicesPanel(
    lrclibEnabled: Boolean,
    lyricsWordFillEnabled: Boolean,
    lastfmEnabled: Boolean,
    lastfmApiKey: String,
    lastfmSecret: String,
    onLrclibToggle: (Boolean) -> Unit,
    onLyricsWordFillToggle: (Boolean) -> Unit,
    onLastfmToggle: (Boolean) -> Unit,
    onLastfmApiKeyChange: (String) -> Unit,
    onLastfmSecretChange: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var showSecret by remember { mutableStateOf(false) }

    FirmiumSettingsRow("External Lyrics (LRCLIB)",
        "Fetch synced lyrics from lrclib.net when your server has none") {
        FirmiumSwitch(checked = lrclibEnabled, onCheckedChange = onLrclibToggle)
    }
    FirmiumSettingsRow("Word-by-Word Lyrics Animation",
        "Karaoke-style fill on the active lyric line, with per-word timing estimated from the line's timestamps") {
        FirmiumSwitch(checked = lyricsWordFillEnabled, onCheckedChange = onLyricsWordFillToggle)
    }
    FirmiumSettingsRow("Last.fm Integration",
        "Fetch artist biography and photo via Last.fm") {
        FirmiumSwitch(checked = lastfmEnabled, onCheckedChange = onLastfmToggle)
    }
    if (lastfmEnabled) {
        Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)) {
            FirmiumTextField(
                value = lastfmApiKey,
                onValueChange = onLastfmApiKeyChange,
                label = "Last.fm API Key",
                modifier = Modifier.fillMaxWidth(),
            )
            FirmiumTextField(
                value = lastfmSecret,
                onValueChange = onLastfmSecretChange,
                label = "Last.fm Secret",
                visualTransformation = if (showSecret) VisualTransformation.None else PasswordVisualTransformation(),
                trailingIcon = {
                    Box(modifier = Modifier.size(36.dp).clip(CircleShape)
                        .clickable { showSecret = !showSecret },
                        contentAlignment = Alignment.Center) {
                        FirmiumIcon(
                            if (showSecret) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                            contentDescription = null, tint = colors.muted, modifier = Modifier.size(18.dp))
                    }
                },
                modifier = Modifier.fillMaxWidth(),
            )
        }
        FirmiumDivider()
    }

    FirmiumListenBrainzSection()
}

// Self-contained ListenBrainz settings — token lives in SecureStorage; the Rust/Android
// scrobbler treats an absent token as disabled, so toggling off removes it.
@Composable
private fun FirmiumListenBrainzSection() {
    val context = LocalContext.current
    val prefs = remember { AppPreferences(context) }
    val secure = remember { SecureStorage(context) }
    val scope = rememberCoroutineScope()
    val colors = LocalFirmiumColors.current
    val enabled by prefs.listenbrainzEnabled.collectAsState(initial = false)
    var token by remember { mutableStateOf(secure.get("listenbrainz", "token") ?: "") }
    var showToken by remember { mutableStateOf(false) }

    FirmiumSettingsRow("ListenBrainz Scrobbling",
        "Submit each completed track to ListenBrainz using your user token") {
        FirmiumSwitch(checked = enabled, onCheckedChange = { v ->
            scope.launch { prefs.setListenbrainzEnabled(v) }
            if (!v) secure.delete("listenbrainz", "token")
            else if (token.isNotBlank()) secure.save("listenbrainz", "token", token)
        })
    }
    if (enabled) {
        Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)) {
            FirmiumTextField(
                value = token,
                onValueChange = { token = it; secure.save("listenbrainz", "token", it) },
                label = "ListenBrainz Token",
                visualTransformation = if (showToken) VisualTransformation.None else PasswordVisualTransformation(),
                trailingIcon = {
                    Box(modifier = Modifier.size(36.dp).clip(CircleShape)
                        .clickable { showToken = !showToken },
                        contentAlignment = Alignment.Center) {
                        FirmiumIcon(
                            if (showToken) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                            contentDescription = null, tint = colors.muted, modifier = Modifier.size(18.dp))
                    }
                },
                modifier = Modifier.fillMaxWidth(),
            )
        }
        FirmiumDivider()
    }
}

@Composable
private fun FirmiumAccountPanel(
    serverUrl: String,
    username: String,
    autoLoginEnabled: Boolean,
    onAutoLoginToggle: (Boolean) -> Unit,
    onLogout: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 14.dp)) {
        Text(serverUrl, fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.text)
        Spacer(Modifier.height(2.dp))
        Text("Logged in as $username", fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
    }
    FirmiumDivider()
    FirmiumSettingsRow("Auto-Login",
        "Automatically connect on startup when credentials are saved") {
        FirmiumSwitch(checked = autoLoginEnabled, onCheckedChange = onAutoLoginToggle)
    }
    Row(
        modifier = Modifier.fillMaxWidth()
            .clickable { onLogout() }
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Text("Disconnect server", fontSize = 15.sp, fontFamily = FontFamily.Monospace,
            color = colors.error)
    }
    FirmiumDivider()
}

@Composable
private fun FirmiumAboutPanel(
    appVersion: String,
    onWipeCache: () -> Unit,
    onClearCache: () -> Unit,
    onResetSettings: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var wipeCacheLabel by remember { mutableStateOf("Wipe") }
    var clearCacheLabel by remember { mutableStateOf("Clear") }
    var resetLabel by remember { mutableStateOf("Reset") }

    FirmiumSettingsRow("App Version", appVersion) {}
    FirmiumSettingsRow("Wipe Cache", "Clear in-memory and disk cover art cache") {
        Box(modifier = Modifier.clip(RoundedCornerShape(4.dp)).clickable {
            onWipeCache(); wipeCacheLabel = "Wiped!"
        }.padding(horizontal = 12.dp, vertical = 6.dp)) {
            Text(wipeCacheLabel, fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.accent)
        }
    }
    FirmiumSettingsRow("Clear Cache", "Remove all cached app data from disk") {
        Box(modifier = Modifier.clip(RoundedCornerShape(4.dp)).clickable {
            onClearCache(); clearCacheLabel = "Cleared!"
        }.padding(horizontal = 12.dp, vertical = 6.dp)) {
            Text(clearCacheLabel, fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.error)
        }
    }
    FirmiumSettingsRow("Reset Settings", "Reset all preferences to defaults") {
        Box(modifier = Modifier.clip(RoundedCornerShape(4.dp)).clickable {
            onResetSettings(); resetLabel = "Done!"
        }.padding(horizontal = 12.dp, vertical = 6.dp)) {
            Text(resetLabel, fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.error)
        }
    }
}

// ── Shared helpers ─────────────────────────────────────────────────────────────

// Dropdown selector matching the desktop version — shows current theme in a bordered row,
// expands to an inline scrollable list when tapped. Closes after selection.
@Composable
private fun ThemeDropdown(currentThemeId: String, onThemeSelected: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    val currentTheme = ALL_THEMES.find { it.id == currentThemeId } ?: ALL_THEMES.first()
    var expanded by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxWidth()) {
        // Trigger row — shows selected theme name + colour swatches
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(6.dp))
                .border(1.dp, colors.border, RoundedCornerShape(6.dp))
                .clickable { expanded = !expanded }
                .padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            // Colour preview swatches
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                Box(Modifier.size(12.dp).clip(CircleShape).background(currentTheme.bg).border(0.5.dp, colors.border, CircleShape))
                Box(Modifier.size(12.dp).clip(CircleShape).background(currentTheme.surface2))
                Box(Modifier.size(12.dp).clip(CircleShape).background(currentTheme.text).border(0.5.dp, colors.border, CircleShape))
                Box(Modifier.size(12.dp).clip(CircleShape).background(currentTheme.accent))
            }
            Text(currentTheme.name, fontSize = 14.sp, fontFamily = FontFamily.Monospace,
                color = colors.text, modifier = Modifier.weight(1f))
            FirmiumIcon(
                if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = null, tint = colors.muted, modifier = Modifier.size(18.dp),
            )
        }

        // Expanded theme list
        AnimatedVisibility(visible = expanded) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(bottomStart = 6.dp, bottomEnd = 6.dp))
                    .border(1.dp, colors.border, RoundedCornerShape(bottomStart = 6.dp, bottomEnd = 6.dp))
                    .background(colors.surface),
            ) {
                ALL_THEMES.forEachIndexed { i, theme ->
                    if (i > 0) FirmiumDivider(color = colors.border)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onThemeSelected(theme.id); expanded = false }
                            .background(if (theme.id == currentThemeId) colors.surface2.copy(alpha = 0.5f) else Color.Transparent)
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Row(horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                            Box(Modifier.size(12.dp).clip(CircleShape).background(theme.bg).border(0.5.dp, colors.border, CircleShape))
                            Box(Modifier.size(12.dp).clip(CircleShape).background(theme.surface2))
                            Box(Modifier.size(12.dp).clip(CircleShape).background(theme.text).border(0.5.dp, colors.border, CircleShape))
                            Box(Modifier.size(12.dp).clip(CircleShape).background(theme.accent))
                        }
                        Text(
                            theme.name, fontSize = 13.sp, fontFamily = FontFamily.Monospace,
                            color = if (theme.id == currentThemeId) colors.accent else colors.text,
                            modifier = Modifier.weight(1f),
                        )
                        if (theme.id == currentThemeId) {
                            FirmiumIcon(Icons.Default.Check, contentDescription = null,
                                tint = colors.accent, modifier = Modifier.size(14.dp))
                        }
                    }
                }
            }
        }
    }
}
