package app.typeaside.keyboard

import android.view.MotionEvent
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Stop
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.input.pointer.pointerInteropFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.exp
import kotlin.math.pow
import kotlin.math.sqrt

sealed interface Bubble {
  data object Idle : Bubble
  data class Recording(val levels: List<Float>) : Bubble
  data class Paused(val levels: List<Float>) : Bubble
  data object Sending : Bubble
  data class Success(val copied: Boolean) : Bubble
  data class Error(val message: String) : Bubble
  data class Limit(val detail: String) : Bubble
}

val Bubble.isRecording: Boolean get() = this is Bubble.Recording || this is Bubble.Paused
val Bubble.isBusy: Boolean get() = isRecording || this == Bubble.Sending

const val RECORDING_WIDTH_DP = 232
const val SHADOW_SPACE_DP = 24
private const val PILL_STIFFNESS = 420f
private const val PILL_DAMPING = 32f
private const val BAR_STIFFNESS = 600f
private const val BAR_DAMPING = 30f
private const val BARS = 13
private const val BAR_CENTER = 6
private const val BAR_ENVELOPE = 4f

fun bubbleWidth(state: Bubble, appearance: Appearance, shrunk: Boolean): Int = when {
  state.isRecording -> RECORDING_WIDTH_DP
  state == Bubble.Sending -> 76
  state is Bubble.Success -> if (state.copied) 112 else appearance.height
  state is Bubble.Error || state is Bubble.Limit -> 280
  shrunk -> 12
  else -> appearance.width
}

fun bubbleHeight(state: Bubble, appearance: Appearance, shrunk: Boolean): Int = when {
  state is Bubble.Limit -> 132
  shrunk -> 12
  state == Bubble.Idle -> appearance.height
  else -> maxOf(48, appearance.height)
}

// Colors and motion match src/ui/theme.rs and src/ui/overlay.rs, independent of wallpaper.
private data class OverlayPalette(val pill: Color, val ink: Color, val muted: Color, val border: Color, val shadow: Color, val blue: Color, val red: Color)

@Composable
private fun overlayPalette() = if (isSystemInDarkTheme()) {
  val ink = Color(0xFFF2F2F1)
  OverlayPalette(Color(0xFF18191B), ink, ink.copy(alpha = .58f), ink.copy(alpha = .14f), Color.Black.copy(alpha = .55f), Color(0xFF8AB8E8), Color(0xFFE27A66))
} else {
  val ink = Color(0xFF16171A)
  OverlayPalette(Color.White, ink, Color(0xFF6B6E76), ink.copy(alpha = .10f), Color(0xFF1E325A).copy(alpha = .20f), Color(0xFF4A84C4), Color(0xFFB4473A))
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun MicBubble(state: Bubble, appearance: Appearance, shrunk: Boolean, onTouch: (MotionEvent) -> Boolean, onToggle: () -> Unit, onCancel: () -> Unit, onPause: () -> Unit) {
  Box(Modifier.fillMaxSize().pointerInteropFilter(onTouchEvent = onTouch).padding(bottom = SHADOW_SPACE_DP.dp), contentAlignment = Alignment.BottomCenter) {
    BubbleSurface(state, appearance, shrunk, onToggle, onCancel, onPause)
  }
}

@Composable
fun BubbleSurface(state: Bubble, appearance: Appearance, shrunk: Boolean = false, onToggle: () -> Unit = {}, onCancel: () -> Unit = {}, onPause: () -> Unit = {}) {
  val palette = overlayPalette()
  val pillSpring = spring<Dp>(stiffness = PILL_STIFFNESS, dampingRatio = PILL_DAMPING / (2 * sqrt(PILL_STIFFNESS)))
  val width by animateDpAsState(bubbleWidth(state, appearance, shrunk).dp, pillSpring, label = "pill width")
  val height by animateDpAsState(bubbleHeight(state, appearance, shrunk).dp, pillSpring, label = "pill height")
  val radius by animateDpAsState(if (state is Bubble.Limit) 12.dp else height / 2, pillSpring, label = "pill radius")
  val shape = RoundedCornerShape(radius)
  Box(
    Modifier.size(width, height)
      .dropShadow(shape, Shadow(radius = 28.dp, color = palette.shadow, offset = DpOffset(0.dp, 10.dp)))
      .clip(shape).background(if (shrunk) palette.ink else palette.pill).border(1.dp, palette.border, shape)
      .semantics {
        if (!state.isRecording) {
          contentDescription = when (state) {
            Bubble.Sending -> "TypeAside is transcribing"
            is Bubble.Success -> if (state.copied) "Dictation copied" else "Dictation inserted"
            is Bubble.Error -> state.message
            is Bubble.Limit -> "Weekly limit reached. ${state.detail}"
            else -> "Dictate with TypeAside"
          }
          if (state == Bubble.Idle || state is Bubble.Error) { role = Role.Button; onClick { onToggle(); true } }
        }
      },
    contentAlignment = Alignment.Center,
  ) {
    if (!shrunk) when (state) {
      Bubble.Idle -> Icon(painterResource(R.drawable.typeaside_mark), null, Modifier.size(24.dp), tint = palette.ink)
      is Bubble.Recording, is Bubble.Paused, Bubble.Sending -> Row(verticalAlignment = Alignment.CenterVertically) {
        if (state != Bubble.Sending) OverlayControl("Cancel dictation", onCancel) { Icon(Icons.Rounded.Close, null, Modifier.size(20.dp), tint = palette.muted) }
        Box(Modifier.width(if (state == Bubble.Sending) 65.dp else 136.dp), contentAlignment = Alignment.Center) {
          if (state is Bubble.Paused) Text("Ⅱ Paused", color = palette.ink, fontSize = 13.sp)
          else Waveform(state, palette)
          if (state != Bubble.Sending) Box(Modifier.align(Alignment.CenterEnd)) {
            OverlayControl(if (state is Bubble.Paused) "Resume dictation" else "Pause dictation", onPause) {
              Icon(if (state is Bubble.Paused) Icons.Rounded.PlayArrow else Icons.Rounded.Pause, null, Modifier.size(20.dp), tint = palette.muted)
            }
          }
        }
        if (state != Bubble.Sending) OverlayControl("Stop dictation", onToggle) { Icon(Icons.Rounded.Stop, null, Modifier.size(22.dp), tint = palette.ink) }
      }
      is Bubble.Success -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        CopiedCheck(palette.ink)
        if (state.copied) Text("Copied", color = palette.ink, fontSize = 13.sp)
      }
      is Bubble.Error -> Row(Modifier.padding(horizontal = 14.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Box(Modifier.size(6.dp).background(palette.red, CircleShape))
        Text(state.message, color = palette.ink, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
      }
      is Bubble.Limit -> Column(Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
          Text("Weekly limit reached", color = palette.ink, fontSize = 13.sp, fontWeight = FontWeight.Medium)
          Text("100%", color = palette.muted, fontSize = 12.sp)
        }
        Box(Modifier.fillMaxWidth().height(4.dp).background(palette.red, CircleShape))
        Text(state.detail, color = palette.muted, fontSize = 12.sp)
        Box(Modifier.fillMaxWidth().height(48.dp).semantics { role = Role.Button; contentDescription = "Dismiss limit notice"; onClick { onCancel(); true } }, contentAlignment = Alignment.CenterEnd) {
          Text("Dismiss", color = palette.ink, fontSize = 13.sp)
        }
      }
    }
  }
}

@Composable
private fun OverlayControl(label: String, action: () -> Unit, content: @Composable () -> Unit) {
  Box(Modifier.size(48.dp).semantics { role = Role.Button; contentDescription = label; onClick { action(); true } }, contentAlignment = Alignment.Center) { content() }
}

@Composable
private fun Waveform(state: Bubble, palette: OverlayPalette) {
  val listening = state is Bubble.Recording
  val levels = (state as? Bubble.Recording)?.levels.orEmpty().takeLast(BARS)
  val bars = List(BARS) { index ->
    val level = levels.getOrNull(index - (BARS - levels.size))?.takeIf { it.isFinite() }?.coerceIn(0f, 1f) ?: 0f
    val target = if (listening) (22f * level * exp(-((index - BAR_CENTER) / BAR_ENVELOPE).pow(2))).coerceAtLeast(3f) else 3f
    val height by animateDpAsState(target.dp, spring(stiffness = BAR_STIFFNESS, dampingRatio = BAR_DAMPING / (2 * sqrt(BAR_STIFFNESS))), label = "bar height")
    val x by animateDpAsState(((if (listening) index else index.coerceIn(3, 9)) - BAR_CENTER).times(5).dp, spring(stiffness = PILL_STIFFNESS, dampingRatio = PILL_DAMPING / (2 * sqrt(PILL_STIFFNESS))), label = "bar position")
    val alpha by animateFloatAsState(if (listening || index in 3..9) 1f else 0f, tween(150), label = "bar fade")
    Triple(height, x, alpha)
  }
  Canvas(Modifier.size(65.dp, 36.dp)) {
    bars.forEach { (height, x, alpha) ->
      drawRoundRect(if (listening) palette.ink.copy(alpha = alpha) else palette.blue.copy(alpha = alpha), Offset(center.x + x.toPx() - 1.5.dp.toPx(), center.y - height.toPx() / 2), Size(3.dp.toPx(), height.toPx()), CornerRadius(1.5.dp.toPx()))
    }
  }
}

@Composable
private fun CopiedCheck(ink: Color) {
  var target by remember { mutableFloatStateOf(0f) }
  LaunchedEffect(Unit) { target = 1f }
  val progress by animateFloatAsState(target, tween(220), label = "copied check")
  Canvas(Modifier.size(18.dp)) {
    val first = Offset(center.x - 6.dp.toPx(), center.y)
    val second = Offset(center.x - 2.dp.toPx(), center.y + 4.dp.toPx())
    val last = Offset(center.x + 6.dp.toPx(), center.y - 5.dp.toPx())
    val path = Path().apply {
      moveTo(first.x, first.y)
      if (progress < .32f) {
        val end = first + (second - first) * (progress / .32f); lineTo(end.x, end.y)
      } else {
        lineTo(second.x, second.y)
        val end = second + (last - second) * ((progress - .32f) / .68f); lineTo(end.x, end.y)
      }
    }
    drawPath(path, ink, style = Stroke(2.dp.toPx(), cap = StrokeCap.Round))
  }
}
