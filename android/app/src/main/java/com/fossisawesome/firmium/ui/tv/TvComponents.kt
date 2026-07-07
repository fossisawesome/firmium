package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.tv.material3.Card
import androidx.tv.material3.CardDefaults
import androidx.tv.material3.Button
import androidx.tv.material3.ButtonDefaults
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.FirmiumColors

// D-pad-focusable tile — used for every album/artist/playlist/track card on TV. Colors
// come from the active Firmium theme (not a MaterialTheme) so focus visuals match the
// rest of the app; androidx.tv.material3 handles the focus scale/border/glow animation.
@Composable
fun TvTile(
    onClick: () -> Unit,
    colors: FirmiumColors,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Card(
        onClick = onClick,
        colors = CardDefaults.colors(
            containerColor = colors.surface,
            contentColor = colors.text,
            focusedContainerColor = colors.surface2,
            focusedContentColor = colors.text,
        ),
        modifier = modifier,
    ) {
        content()
    }
}

// D-pad-focusable action button — transport controls, queue toggle, login submit.
@Composable
fun TvActionButton(
    onClick: () -> Unit,
    colors: FirmiumColors,
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = ButtonDefaults.ContentPadding,
    content: @Composable () -> Unit,
) {
    Button(
        onClick = onClick,
        colors = ButtonDefaults.colors(
            containerColor = colors.surface2,
            contentColor = colors.text,
            focusedContainerColor = colors.accent,
            focusedContentColor = colors.bg,
        ),
        contentPadding = contentPadding,
        modifier = modifier,
    ) {
        content()
    }
}

// A labeled row with an on/off toggle — whole row is one focusable button (no drag, just select).
@Composable
fun TvToggleRow(
    label: String,
    checked: Boolean,
    colors: FirmiumColors,
    onToggle: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    TvActionButton(onClick = { onToggle(!checked) }, colors = colors, modifier = modifier.fillMaxWidth()) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(text = label, color = colors.text, fontSize = 14.sp)
            Text(text = if (checked) "On" else "Off", color = if (checked) colors.accent else colors.muted, fontSize = 14.sp)
        }
    }
}

// A labeled row that cycles through a fixed list of options via prev/next buttons —
// the D-pad-friendly stand-in for a dropdown/spinner (no touch menu on a TV remote).
@Composable
fun TvCycleRow(
    label: String,
    options: List<String>,
    selectedIndex: Int,
    colors: FirmiumColors,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(modifier = modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(text = label, color = colors.text, fontSize = 14.sp, modifier = Modifier.weight(1f))
        TvActionButton(onClick = { onSelect((selectedIndex - 1 + options.size) % options.size) }, colors = colors) {
            Text(text = "<", color = colors.text, fontSize = 14.sp)
        }
        Text(
            text = options.getOrElse(selectedIndex) { "" },
            color = colors.accent,
            fontSize = 13.sp,
            maxLines = 1,
            modifier = Modifier.padding(horizontal = 12.dp),
        )
        TvActionButton(onClick = { onSelect((selectedIndex + 1) % options.size) }, colors = colors) {
            Text(text = ">", color = colors.text, fontSize = 14.sp)
        }
    }
}

// A labeled row with a numeric value adjusted by +/- buttons — the D-pad-friendly stand-in
// for a drag slider (crossfade duration, EQ band gain), since a TV remote can't drag.
@Composable
fun TvStepperRow(
    label: String,
    valueText: String,
    colors: FirmiumColors,
    onDecrement: () -> Unit,
    onIncrement: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(modifier = modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(text = label, color = colors.text, fontSize = 14.sp, modifier = Modifier.weight(1f))
        TvActionButton(onClick = onDecrement, colors = colors) {
            Text(text = "-", color = colors.text, fontSize = 14.sp)
        }
        Text(text = valueText, color = colors.accent, fontSize = 13.sp, modifier = Modifier.padding(horizontal = 12.dp))
        TvActionButton(onClick = onIncrement, colors = colors) {
            Text(text = "+", color = colors.text, fontSize = 14.sp)
        }
    }
}
