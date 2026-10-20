package app.typeaside.keyboard

import android.os.Build
import android.view.MotionEvent
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ErrorOutline
import androidx.compose.material.icons.rounded.Mic
import androidx.compose.material.icons.rounded.Stop
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInteropFilter
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.unit.dp

sealed interface Bubble {
  data object Idle : Bubble
  data class Recording(val level: Float) : Bubble
  data object Sending : Bubble
  data object Error : Bubble
}

// The window is larger than the bubble so the level halo and shadow are not clipped.
const val BUBBLE_WINDOW_DP = 72
private val BubbleSize = 52.dp

@Composable
fun BubbleTheme(content: @Composable () -> Unit) {
  val context = LocalContext.current
  val dark = isSystemInDarkTheme()
  val scheme = when {
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    dark -> darkColorScheme()
    else -> lightColorScheme()
  }
  MaterialTheme(colorScheme = scheme, content = content)
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun MicBubble(state: Bubble, onTouch: (MotionEvent) -> Boolean, onToggle: () -> Unit) {
  val colors = MaterialTheme.colorScheme
  val level by animateFloatAsState(if (state is Bubble.Recording) state.level else 0f, label = "level")
  val (container, content) = when (state) {
    Bubble.Idle -> colors.primaryContainer to colors.onPrimaryContainer
    is Bubble.Recording -> colors.primary to colors.onPrimary
    Bubble.Sending -> colors.surfaceContainerHigh to colors.primary
    Bubble.Error -> colors.errorContainer to colors.onErrorContainer
  }
  Box(
    Modifier
      .size(BUBBLE_WINDOW_DP.dp)
      .pointerInteropFilter(onTouchEvent = onTouch)
      .clearAndSetSemantics {
        role = Role.Button
        contentDescription = if (state is Bubble.Recording) "Stop dictation" else "Dictate"
        onClick {
          onToggle()
          true
        }
      },
    contentAlignment = Alignment.Center,
  ) {
    if (state is Bubble.Recording) {
      Box(
        Modifier
          .size(BubbleSize)
          .graphicsLayer {
            scaleX = 1f + level * 0.36f
            scaleY = 1f + level * 0.36f
          }
          .background(colors.primary.copy(alpha = 0.3f), CircleShape),
      )
    }
    Surface(Modifier.size(BubbleSize), shape = CircleShape, color = container, contentColor = content, shadowElevation = 3.dp) {
      Box(contentAlignment = Alignment.Center) {
        when (state) {
          Bubble.Idle -> Icon(Icons.Rounded.Mic, contentDescription = null)
          is Bubble.Recording -> Icon(Icons.Rounded.Stop, contentDescription = null)
          Bubble.Sending -> CircularProgressIndicator(Modifier.size(24.dp), color = content, strokeWidth = 3.dp)
          Bubble.Error -> Icon(Icons.Rounded.ErrorOutline, contentDescription = null)
        }
      }
    }
  }
}
