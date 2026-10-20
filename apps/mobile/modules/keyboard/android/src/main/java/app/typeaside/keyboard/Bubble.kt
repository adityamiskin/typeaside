package app.typeaside.keyboard

import android.os.Build
import android.view.MotionEvent
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.ErrorOutline
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInteropFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.unit.dp

sealed interface Bubble {
  data object Idle : Bubble
  data class Recording(val levels: List<Float>) : Bubble
  data object Sending : Bubble
  data object Error : Bubble
}

const val RECORDING_WIDTH_DP = 232

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
fun MicBubble(state: Bubble, appearance: Appearance, shrunk: Boolean, onTouch: (MotionEvent) -> Boolean, onToggle: () -> Unit, onCancel: () -> Unit) {
  val colors = MaterialTheme.colorScheme
  val recording = state is Bubble.Recording
  val width = if (recording) RECORDING_WIDTH_DP else if (shrunk) 12 else appearance.width
  Box(
    Modifier.size(maxOf(width, 48).dp, (appearance.height + 16).dp)
      .pointerInteropFilter(onTouchEvent = onTouch)
      .clearAndSetSemantics {
        role = Role.Button
        contentDescription = when (state) {
          is Bubble.Recording -> "Stop TypeAside dictation"
          Bubble.Sending -> "TypeAside is transcribing"
          else -> "Dictate with TypeAside"
        }
        if (state != Bubble.Sending) onClick { onToggle(); true }
        if (recording) customActions = listOf(CustomAccessibilityAction("Cancel dictation") { onCancel(); true })
      },
    contentAlignment = Alignment.Center,
  ) {
    Surface(
      Modifier.size(width.dp, (if (shrunk) 12 else appearance.height).dp), shape = CircleShape,
      color = if (shrunk) colors.primary else if (state == Bubble.Error) colors.errorContainer else colors.secondaryContainer,
      contentColor = if (state == Bubble.Error) colors.onErrorContainer else colors.onSecondaryContainer,
      shadowElevation = 2.dp,
    ) {
      if (!shrunk) Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceEvenly) {
        when (state) {
          is Bubble.Recording -> {
            Icon(Icons.Rounded.Close, contentDescription = null, modifier = Modifier.size(20.dp))
            Row(Modifier.width(116.dp), horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.CenterVertically) {
              repeat(20) { index ->
                val level = state.levels.getOrNull(index - (20 - state.levels.size)) ?: 0f
                Box(Modifier.size(3.dp, (3 + level * 22).dp).background(colors.onSecondaryContainer, CircleShape))
              }
            }
            Icon(Icons.Rounded.Stop, contentDescription = null, modifier = Modifier.size(24.dp))
          }
          Bubble.Sending -> CircularProgressIndicator(Modifier.size(18.dp), color = colors.onSecondaryContainer, strokeWidth = 2.dp)
          Bubble.Error -> Icon(Icons.Rounded.ErrorOutline, contentDescription = null, modifier = Modifier.size(22.dp))
          Bubble.Idle -> Icon(painterResource(R.drawable.typeaside_mark), contentDescription = null, modifier = Modifier.size(24.dp))
        }
      }
    }
  }
}
