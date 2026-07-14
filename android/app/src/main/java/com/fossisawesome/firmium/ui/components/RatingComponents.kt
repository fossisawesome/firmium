package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// 1-5 star rating row. Tapping the current rating clears it (via caller logic).
// Also reused, unmodified, as the rating-filter control on the search screen —
// the "rating" being shown is just whatever int the caller passes in.
@Composable
fun StarRating(
    rating: Int,
    onRate: (Int) -> Unit,
    starSize: Dp,
    accentColor: Color,
    mutedColor: Color,
) {
    Row(
        // No extra gap: each star already reserves a 44dp tap box below, so touching
        // boxes give ample separation without inflating the row's total width further.
        horizontalArrangement = Arrangement.spacedBy(0.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        for (i in 1..5) {
            // Tap target is fixed at 44dp regardless of the visually-rendered star size,
            // so compact stars (e.g. search screen's 16-18dp) still meet touch-target guidelines.
            Box(
                modifier = Modifier.size(44.dp).clickable { onRate(i) },
                contentAlignment = Alignment.Center,
            ) {
                FirmiumIcon(
                    imageVector = if (i <= rating) Icons.Default.Star else Icons.Default.StarBorder,
                    contentDescription = "Rate $i",
                    tint = if (i <= rating) accentColor else mutedColor,
                    modifier = Modifier.size(starSize),
                )
            }
        }
    }
}

// Read-only community average rating (OpenSubsonic averageRating). Hidden when null.
@Composable
fun AvgRatingBadge(
    rating: Double?,
    starSize: Dp,
    mutedColor: Color,
) {
    if (rating == null || rating <= 0.0) return
    Row(
        horizontalArrangement = Arrangement.spacedBy(2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        FirmiumIcon(
            imageVector = Icons.Default.Star,
            contentDescription = "Average rating",
            tint = mutedColor,
            modifier = Modifier.size(starSize),
        )
        Text(
            text = "%.1f".format(rating),
            fontSize = 11.sp,
            fontFamily = LocalAppFontFamily.current,
            color = mutedColor,
        )
    }
}
