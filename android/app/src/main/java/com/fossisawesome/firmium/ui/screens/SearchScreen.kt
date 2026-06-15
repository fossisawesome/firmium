package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.PlaylistAdd
import androidx.compose.material.icons.filled.Search
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.SearchState

// Search screen — Firmium styling, exact port of MobileSearch.svelte.
@Composable
fun SearchScreen(
    state: SearchState,
    coverUrlFor: (String?) -> String?,
    playlists: List<Playlist>,
    onBack: () -> Unit,
    onQueryChange: (String) -> Unit,
    onSearch: () -> Unit,
    onPlaySong: (List<Song>, Int) -> Unit,
    onAlbumClick: (String) -> Unit,
    onAddSongToPlaylist: (playlistId: String, song: Song) -> Unit,
    onCreatePlaylistAndAddSong: (name: String, song: Song) -> Unit,
    onAddAlbum: (albumId: String) -> Unit = {},
    onDownloadAlbum: ((Album) -> suspend () -> Result<Unit>)? = null,
    onDownloadTrack: ((Song) -> suspend () -> Result<Unit>)? = null,
) {
    val colors = LocalFirmiumColors.current
    val border = colors.surface2.copy(alpha = 0.4f)
    var pendingSong by remember { mutableStateOf<Song?>(null) }
    val focusRequester = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current

    LaunchedEffect(Unit) { focusRequester.requestFocus() }

    Column(modifier = Modifier.fillMaxSize().imePadding()) {
        // Header: back arrow + rounded search input + search button
        // Matches .ms-header: padding 12/12/10, border-bottom, gap 8dp
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(horizontal = 12.dp, vertical = 12.dp)
                .padding(bottom = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // Back button — matches .ms-back-btn (44x44dp circle)
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(50))
                    .clickable { onBack() },
                contentAlignment = Alignment.Center,
            ) {
                FirmiumIcon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back",
                    tint = colors.text, modifier = Modifier.size(24.dp))
            }

            // Search input — matches .ms-input: surface bg, border, radius 10dp, 16sp monospace
            BasicTextField(
                value = state.query,
                onValueChange = onQueryChange,
                textStyle = TextStyle(
                    color = colors.text,
                    fontSize = 16.sp,
                    fontFamily = FontFamily.Monospace,
                ),
                cursorBrush = SolidColor(colors.accent),
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { onSearch(); keyboard?.hide() }),
                modifier = Modifier
                    .weight(1f)
                    .focusRequester(focusRequester),
                decorationBox = { inner ->
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(10.dp))
                            .background(colors.surface)
                            .border(1.dp, border, RoundedCornerShape(10.dp))
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                    ) {
                        if (state.query.isEmpty()) {
                            Text("Search albums, songs…",
                                fontSize = 16.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                        }
                        inner()
                    }
                },
            )

            // Search execute button — matches .ms-search-exec (44x44dp, accent color)
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(50))
                    .clickable { onSearch(); keyboard?.hide() },
                contentAlignment = Alignment.Center,
            ) {
                FirmiumIcon(Icons.Default.Search, contentDescription = "Search",
                    tint = colors.accent, modifier = Modifier.size(20.dp))
            }
        }
        // Border-bottom matching .ms-header border-bottom
        FirmiumDivider()

        // Results body
        when {
            state.isLoading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("Searching…", fontSize = 14.sp, color = colors.muted, fontFamily = FontFamily.Monospace)
            }
            state.query.isBlank() -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("Type above and press Search or Enter",
                    fontSize = 14.sp, color = colors.muted, fontFamily = FontFamily.Monospace)
            }
            state.songs.isEmpty() && state.albums.isEmpty() -> Box(
                Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No results found.", fontSize = 14.sp, color = colors.muted, fontFamily = FontFamily.Monospace)
            }
            else -> LazyColumn(modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp)) {
                if (state.songs.isNotEmpty()) {
                    // .section-header: 12sp uppercase, muted, letterSpacing 1, margin 10/12dp
                    item {
                        Text("SONGS", fontSize = 12.sp, color = colors.muted, fontFamily = FontFamily.Monospace,
                            letterSpacing = 1.sp,
                            modifier = Modifier.padding(start = 16.dp, top = 10.dp, bottom = 12.dp))
                    }
                    itemsIndexed(state.songs, key = { _, s -> "song_${s.id}" }) { index, song ->
                        SearchTrackRow(
                            song = song,
                            index = index + 1,
                            coverUrl = coverUrlFor(song.coverArt),
                            isPlaying = false,
                            onClick = { onPlaySong(state.songs, index) },
                            onAddToPlaylist = { pendingSong = song },
                            onDownloadClick = onDownloadTrack?.invoke(song),
                        )
                        FirmiumDivider()
                    }
                }
                if (state.albums.isNotEmpty()) {
                    item {
                        Text("ALBUMS", fontSize = 12.sp, color = colors.muted, fontFamily = FontFamily.Monospace,
                            letterSpacing = 1.sp,
                            modifier = Modifier.padding(start = 16.dp, top = 10.dp, bottom = 12.dp))
                    }
                    itemsIndexed(state.albums, key = { _, a -> "album_${a.id}" }) { _, album ->
                        AlbumRow(
                            album = album,
                            coverUrl = coverUrlFor(album.coverArt),
                            onAlbumClick = onAlbumClick,
                            onAddClick = { onAddAlbum(album.id) },
                            onDownloadClick = onDownloadAlbum?.invoke(album),
                            coverSize = 40.dp,
                            coverRadius = 8.dp,
                        )
                        FirmiumDivider()
                    }
                }
            }
        }
    }

    val song = pendingSong
    if (song != null) {
        AddToPlaylistDialog(
            playlists = playlists,
            onAddTo = { pid -> onAddSongToPlaylist(pid, song); pendingSong = null },
            onCreateAndAdd = { name -> onCreatePlaylistAndAddSong(name, song); pendingSong = null },
            onDismiss = { pendingSong = null },
        )
    }

}

// Track row matching .track-row: padding 10dp, gap 16dp.
@Composable
private fun SearchTrackRow(
    song: Song,
    index: Int,
    coverUrl: String?,
    isPlaying: Boolean,
    onClick: () -> Unit,
    onAddToPlaylist: () -> Unit,
    onDownloadClick: (suspend () -> Result<Unit>)? = null,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(if (isPlaying) colors.accent.copy(alpha = 0.12f) else androidx.compose.ui.graphics.Color.Transparent)
            .clickable { onClick() }
            .padding(10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // Track number — .track-num: 24dp width, 11sp, muted, right-aligned
        Text(
            text = "$index",
            fontSize = 11.sp,
            color = if (isPlaying) colors.accent else colors.muted,
            fontFamily = FontFamily.Monospace,
            modifier = Modifier.width(24.dp),
            textAlign = androidx.compose.ui.text.style.TextAlign.End,
        )
        // Thumbnail — .track-thumb: 36x36, radius 8dp
        CoverImage(
            url = coverUrl,
            contentDescription = null,
            modifier = Modifier.size(36.dp).clip(RoundedCornerShape(8.dp)),
        )
        // Track info — .track-info: flex-1
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = song.title,
                fontSize = 14.sp,
                fontFamily = FontFamily.Monospace,
                color = if (isPlaying) colors.accent else colors.text,
                maxLines = 1,
            )
            Spacer(Modifier.height(2.dp))
            Text(
                text = song.displayArtist ?: song.artist,
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
                color = colors.muted,
                maxLines = 1,
            )
        }
        // Duration — .track-duration: 12sp muted
        Text(
            text = formatDuration(song.duration),
            fontSize = 12.sp,
            color = colors.muted,
            fontFamily = FontFamily.Monospace,
            modifier = Modifier.padding(end = 10.dp),
        )
        if (onDownloadClick != null) {
            DownloadButton(onDownload = onDownloadClick, buttonSize = 26.dp, iconSize = 16.dp)
        }
        // Add-to-playlist button — .track-add-btn: 26x26 circle
        Box(
            modifier = Modifier
                .size(26.dp)
                .clip(RoundedCornerShape(50))
                .clickable { onAddToPlaylist() },
            contentAlignment = Alignment.Center,
        ) {
            FirmiumIcon(Icons.Default.PlaylistAdd, contentDescription = "Add to playlist",
                tint = colors.muted, modifier = Modifier.size(16.dp))
        }
    }
}

private fun formatDuration(seconds: Int): String {
    val m = seconds / 60; val s = seconds % 60
    return "%d:%02d".format(m, s)
}
