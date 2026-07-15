package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Heart toggle for the Favorites feature — filled + accent-colored when starred,
// outline + muted otherwise. Used on album/track/artist headers and the player.
@Composable
fun FavoriteButton(
    starred: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
) {
    val colors = LocalFirmiumColors.current
    FirmiumIconButton(onClick = onToggle, modifier = modifier.size(size + 16.dp)) {
        FirmiumIcon(
            imageVector = if (starred) Icons.Default.Favorite else Icons.Default.FavoriteBorder,
            contentDescription = if (starred) "Remove from favorites" else "Add to favorites",
            tint = if (starred) colors.accent else colors.muted,
            modifier = Modifier.size(size),
        )
    }
}
