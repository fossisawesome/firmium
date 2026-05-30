package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.LyricsState
import kotlinx.coroutines.launch

// Full-screen lyrics sheet. Auto-scrolls to the active line for synced lyrics.
@Composable
fun LyricsSheet(
    state: LyricsState,
    trackTitle: String,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()

    // Auto-scroll to active line whenever it changes.
    LaunchedEffect(state.activeLine) {
        if (state.activeLine >= 0 && state.synced) {
            scope.launch {
                listState.animateScrollToItem(
                    index = (state.activeLine - 2).coerceAtLeast(0),
                )
            }
        }
    }

    FirmiumBottomSheet(onDismiss = onDismiss) {
        Column(
            modifier = Modifier.fillMaxWidth().fillMaxHeight(0.9f),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = trackTitle,
                fontSize = 15.sp,
                fontFamily = FontFamily.Monospace,
                color = colors.text,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
                textAlign = TextAlign.Center,
            )

            FirmiumDivider(
                color = colors.border,
                modifier = Modifier.padding(horizontal = 24.dp),
            )

            when {
                state.isLoading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
                }
                state.lines.isEmpty() -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        "No lyrics available",
                        fontSize = 14.sp,
                        fontFamily = FontFamily.Monospace,
                        fontStyle = FontStyle.Italic,
                        color = colors.muted,
                    )
                }
                else -> LazyColumn(
                    state = listState,
                    contentPadding = PaddingValues(horizontal = 24.dp, vertical = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(if (state.synced) 12.dp else 4.dp),
                    modifier = Modifier.fillMaxSize(),
                ) {
                    itemsIndexed(state.lines, key = { i, _ -> i }) { index, line ->
                        val isActive = state.synced && index == state.activeLine
                        val isNear = state.synced && (index == state.activeLine - 1 || index == state.activeLine + 1)
                        Text(
                            text = line.text.ifBlank { "​" }, // zero-width space preserves blank line height
                            fontSize = if (isActive) 16.sp else 14.sp,
                            fontFamily = FontFamily.Monospace,
                            color = when {
                                isActive -> colors.accent
                                isNear -> colors.text.copy(alpha = 0.7f)
                                else -> colors.muted.copy(alpha = 0.5f)
                            },
                            textAlign = TextAlign.Center,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                }
            }
        }
    }
}
