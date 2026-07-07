package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.ui.components.CoverImage
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.HomeState
import com.fossisawesome.firmium.viewmodel.RecentArtist

@Composable
fun TvHomeScreen(
    state: HomeState,
    coverUrlFor: (String?) -> String?,
    onLoad: () -> Unit,
    onAlbumClick: (String) -> Unit,
    onArtistClick: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val firstFocusRequester = remember { FocusRequester() }

    LaunchedEffect(Unit) { onLoad() }
    LaunchedEffect(state.recentAlbums) {
        if (state.recentAlbums.isNotEmpty()) firstFocusRequester.requestFocus()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(vertical = 24.dp),
    ) {
        if (state.recentAlbums.isNotEmpty()) {
            TvShelf(
                title = "Recently Added",
                albums = state.recentAlbums,
                coverUrlFor = coverUrlFor,
                onAlbumClick = onAlbumClick,
                firstItemFocusRequester = firstFocusRequester,
            )
        }
        if (state.recentArtists.isNotEmpty()) {
            TvArtistShelf(
                title = "Artists",
                artists = state.recentArtists,
                coverUrlFor = coverUrlFor,
                onArtistClick = onArtistClick,
            )
        }
        if (state.randomAlbums.isNotEmpty()) {
            TvShelf(
                title = "Random Picks",
                albums = state.randomAlbums,
                coverUrlFor = coverUrlFor,
                onAlbumClick = onAlbumClick,
            )
        }
    }
}

@Composable
private fun TvShelf(
    title: String,
    albums: List<Album>,
    coverUrlFor: (String?) -> String?,
    onAlbumClick: (String) -> Unit,
    firstItemFocusRequester: FocusRequester? = null,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.padding(bottom = 32.dp)) {
        Text(
            text = title,
            color = colors.text,
            fontSize = 20.sp,
            modifier = Modifier.padding(start = 48.dp, bottom = 12.dp),
        )
        LazyRow(
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 48.dp),
            horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(16.dp),
        ) {
            itemsIndexed(albums) { index, album ->
                TvTile(
                    onClick = { onAlbumClick(album.id) },
                    colors = colors,
                    modifier = Modifier
                        .size(width = 160.dp, height = 200.dp)
                        .then(
                            if (index == 0 && firstItemFocusRequester != null)
                                Modifier.focusRequester(firstItemFocusRequester)
                            else Modifier
                        ),
                ) {
                    Column {
                        CoverImage(
                            url = coverUrlFor(album.coverArt),
                            contentDescription = album.name,
                            size = 160.dp,
                        )
                        Text(
                            text = album.name,
                            color = colors.text,
                            fontSize = 13.sp,
                            maxLines = 1,
                            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
                        )
                        Text(
                            text = album.artist,
                            color = colors.muted,
                            fontSize = 11.sp,
                            maxLines = 1,
                            modifier = Modifier.padding(horizontal = 4.dp),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun TvArtistShelf(
    title: String,
    artists: List<RecentArtist>,
    coverUrlFor: (String?) -> String?,
    onArtistClick: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.padding(bottom = 32.dp)) {
        Text(
            text = title,
            color = colors.text,
            fontSize = 20.sp,
            modifier = Modifier.padding(start = 48.dp, bottom = 12.dp),
        )
        LazyRow(
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 48.dp),
            horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(16.dp),
        ) {
            items(artists) { artist ->
                TvTile(
                    onClick = { onArtistClick(artist.id) },
                    colors = colors,
                    modifier = Modifier.size(width = 140.dp, height = 180.dp),
                ) {
                    Column {
                        CoverImage(
                            url = coverUrlFor(artist.coverArt),
                            contentDescription = artist.name,
                            size = 140.dp,
                        )
                        Text(
                            text = artist.name,
                            color = colors.text,
                            fontSize = 13.sp,
                            maxLines = 1,
                            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
                        )
                    }
                }
            }
        }
    }
}
