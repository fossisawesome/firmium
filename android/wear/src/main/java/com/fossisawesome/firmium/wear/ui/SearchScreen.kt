package com.fossisawesome.firmium.wear.ui

import android.app.Activity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import android.app.RemoteInput
import androidx.compose.ui.Modifier
import androidx.navigation.NavController
import androidx.wear.compose.foundation.lazy.ScalingLazyColumn
import androidx.wear.compose.foundation.lazy.items
import androidx.wear.compose.material.Chip
import androidx.wear.compose.material.ChipDefaults
import androidx.wear.compose.material.CircularProgressIndicator
import androidx.wear.compose.material.ListHeader
import androidx.wear.compose.material.Text
import androidx.wear.input.RemoteInputIntentHelper
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.wear.FirmiumWearApplication

private const val SEARCH_INPUT_KEY = "firmium_search_query"

@Composable
fun SearchScreen(app: FirmiumWearApplication, navController: NavController) {
    var query by remember { mutableStateOf<String?>(null) }
    var results by remember { mutableStateOf<UiState<ApiClient.SearchResults>?>(null) }

    val launcher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            val text = RemoteInput.getResultsFromIntent(result.data)?.getCharSequence(SEARCH_INPUT_KEY)?.toString()
            if (!text.isNullOrBlank()) query = text
        }
    }

    LaunchedEffect(Unit) {
        val intent = RemoteInputIntentHelper.createActionRemoteInputIntent()
        val remoteInputs = listOf(RemoteInput.Builder(SEARCH_INPUT_KEY).setLabel("Search").build())
        RemoteInputIntentHelper.putRemoteInputsExtra(intent, remoteInputs)
        launcher.launch(intent)
    }

    LaunchedEffect(query) {
        val q = query ?: return@LaunchedEffect
        results = UiState.Loading
        results = try {
            UiState.Success(app.api.search(q))
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Search failed")
        }
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        item { ListHeader { Text(query ?: "Search") } }
        when (val state = results) {
            null -> {}
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> {
                items(state.data.songs) { song ->
                    Chip(
                        onClick = {
                            app.playbackController.playAt(state.data.songs, state.data.songs.indexOf(song))
                            navController.navigate("nowPlaying")
                        },
                        label = { Text(song.title, maxLines = 1) },
                        secondaryLabel = { Text(song.displayArtist ?: song.artist, maxLines = 1) },
                        colors = ChipDefaults.secondaryChipColors(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
                items(state.data.albums) { album ->
                    Chip(
                        onClick = { navController.navigate("album/${album.id}") },
                        label = { Text(album.name, maxLines = 1) },
                        secondaryLabel = { Text(album.artist, maxLines = 1) },
                        colors = ChipDefaults.secondaryChipColors(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
        }
    }
}
