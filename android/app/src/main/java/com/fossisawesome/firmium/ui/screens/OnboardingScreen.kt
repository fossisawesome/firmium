package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.components.FirmiumTextButton
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.launch

private data class OnboardPanel(val title: String, val body: String)

private val panels = listOf(
    OnboardPanel("Welcome to Firmium", "Your music, your server."),
    OnboardPanel("Your music, your way", "Connect any OpenSubsonic or Navidrome server, or play your local files. No lock-in, nothing uploaded."),
    OnboardPanel("Built for listening", "Gapless playback and smooth crossfade transitions."),
    OnboardPanel("Make it yours", "Light, dark, or your own custom theme."),
    OnboardPanel("Ready to go", "Connect your server to start listening."),
)

@Composable
fun OnboardingScreen(onFinish: () -> Unit) {
    val colors = LocalFirmiumColors.current
    val pagerState = rememberPagerState(pageCount = { panels.size })
    val scope = rememberCoroutineScope()
    val isLast = pagerState.currentPage == panels.size - 1

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(colors.bg)
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        HorizontalPager(
            state = pagerState,
            modifier = Modifier.weight(1f),
        ) { page ->
            Column(
                modifier = Modifier.fillMaxSize(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                if (page == 0) {
                    HexLogo()
                    Spacer(Modifier.height(24.dp))
                }
                Text(
                    text = panels[page].title,
                    color = colors.text,
                    fontSize = 22.sp,
                    fontWeight = FontWeight.SemiBold,
                    textAlign = TextAlign.Center,
                )
                Spacer(Modifier.height(12.dp))
                Text(
                    text = panels[page].body,
                    color = colors.muted,
                    fontSize = 15.sp,
                    lineHeight = 22.sp,
                    textAlign = TextAlign.Center,
                )
            }
        }

        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.padding(vertical = 16.dp),
        ) {
            repeat(panels.size) { i ->
                val active = i == pagerState.currentPage
                Box(
                    Modifier
                        .size(8.dp)
                        .clip(CircleShape)
                        .background(if (active) colors.accent else colors.border)
                )
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (!isLast) {
                FirmiumTextButton(onClick = onFinish) {
                    Text(text = "Skip", color = colors.muted, fontSize = 14.sp)
                }
            }
            Spacer(Modifier.weight(1f))
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(4.dp))
                    .background(colors.accent)
                    .clickable {
                        if (isLast) onFinish()
                        else scope.launch { pagerState.animateScrollToPage(pagerState.currentPage + 1) }
                    }
                    .padding(horizontal = 20.dp, vertical = 12.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = if (isLast) "Connect your server" else "Next",
                    color = colors.bg,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
    }
}

@Composable
private fun HexLogo() {
    Canvas(modifier = Modifier.size(120.dp)) {
        val w = size.width
        val h = size.height
        // Hexagon points from the 1024-viewBox logo, scaled to the canvas.
        val pts = listOf(
            0.500f to 0.125f, 0.818f to 0.3125f, 0.818f to 0.6875f,
            0.500f to 0.875f, 0.182f to 0.6875f, 0.182f to 0.3125f,
        )
        val path = Path().apply {
            moveTo(pts[0].first * w, pts[0].second * h)
            for (i in 1 until pts.size) lineTo(pts[i].first * w, pts[i].second * h)
            close()
        }
        drawPath(
            path = path,
            brush = Brush.linearGradient(
                colors = listOf(Color(0xFFE8C97E), Color(0xFF863BFF)),
                start = Offset(0f, 0f),
                end = Offset(w, h),
            ),
            style = Stroke(width = w * 0.0547f, join = StrokeJoin.Round),
        )
    }
}
