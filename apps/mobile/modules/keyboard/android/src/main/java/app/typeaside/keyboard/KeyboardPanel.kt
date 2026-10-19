package app.typeaside.keyboard

import android.os.Build
import android.os.SystemClock
import android.view.inputmethod.EditorInfo
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.indication
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.automirrored.rounded.ArrowForward
import androidx.compose.material.icons.automirrored.rounded.Backspace
import androidx.compose.material.icons.automirrored.rounded.KeyboardReturn
import androidx.compose.material.icons.automirrored.rounded.Send
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Language
import androidx.compose.material.icons.rounded.Mic
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material.icons.rounded.SpaceBar
import androidx.compose.material.icons.rounded.Stop
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

sealed interface Panel {
  data object SignedOut : Panel
  data object NoMicrophone : Panel
  data class Ready(val status: String? = null) : Panel
  data class Recording(val seconds: Int, val level: Float) : Panel
  data object Sending : Panel
}

interface Keys {
  fun start()
  fun stop()
  fun openApp()
  fun switchKeyboard()
  fun pickKeyboard()
  fun delete()
  fun space()
  fun enter()
}

// Releasing after this long means the press was push-to-talk rather than a tap that toggles recording.
private const val HOLD_TO_TALK_MS = 400L
private val KeyShape = RoundedCornerShape(12.dp)

@Composable
fun KeyboardTheme(content: @Composable () -> Unit) {
  val context = LocalContext.current
  val scheme = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) dynamicDarkColorScheme(context) else darkColorScheme()
  MaterialTheme(
    colorScheme = scheme.copy(
      background = Color.Black,
      surface = Color.Black,
      onBackground = Color.White,
      onSurface = Color.White,
    ),
    content = content,
  )
}

@Composable
fun KeyboardPanel(panel: Panel, returnAction: Int?, keys: Keys) {
  Column(
    Modifier
      .fillMaxWidth()
      .background(Color.Black)
      .windowInsetsPadding(WindowInsets.navigationBars)
      .padding(horizontal = 8.dp, vertical = 8.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
  ) {
    Status(panel)
    Box(Modifier.height(136.dp), contentAlignment = Alignment.Center) {
      when (panel) {
        Panel.SignedOut, Panel.NoMicrophone -> FilledTonalButton(onClick = keys::openApp) { Text("Open TypeAside") }
        else -> MicButton(panel, keys)
      }
    }
    Row(Modifier.fillMaxWidth().height(52.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
      Key(Icons.Rounded.Language, "Switch keyboard", Modifier.width(64.dp), onLongClick = keys::pickKeyboard, onClick = keys::switchKeyboard)
      Key(Icons.Rounded.SpaceBar, "Space", Modifier.weight(1f), onClick = keys::space)
      BackspaceKey(keys::delete)
      Key(
        returnIcon(returnAction),
        "Return",
        Modifier.width(64.dp),
        highlighted = returnAction != null,
        onClick = keys::enter,
      )
    }
  }
}

@Composable
private fun Status(panel: Panel) {
  val (text, color) = when (panel) {
    Panel.SignedOut -> "Open TypeAside to sign in" to Color.White
    Panel.NoMicrophone -> "Allow microphone access in TypeAside" to Color.White
    is Panel.Ready -> (panel.status ?: "Tap or hold to talk") to (if (panel.status == null) Color.White.copy(alpha = 0.6f) else Color.White)
    is Panel.Recording -> "%d:%02d".format(panel.seconds / 60, panel.seconds % 60) to MaterialTheme.colorScheme.error
    Panel.Sending -> "Transcribing" to Color.White
  }
  Text(text, color = color, fontSize = 14.sp, fontWeight = FontWeight.Medium, maxLines = 2, modifier = Modifier.padding(top = 4.dp))
}

@Composable
private fun MicButton(panel: Panel, keys: Keys) {
  val recording = panel is Panel.Recording
  val sending = panel is Panel.Sending
  val haptics = LocalHapticFeedback.current
  val isRecording by rememberUpdatedState(recording)
  val colors = MaterialTheme.colorScheme
  val level by animateFloatAsState(if (panel is Panel.Recording) panel.level else 0f, label = "level")
  val interaction = remember { MutableInteractionSource() }

  Box(contentAlignment = Alignment.Center) {
    Box(
      Modifier
        .size(88.dp)
        .graphicsLayer {
          scaleX = 1f + level * 0.4f
          scaleY = 1f + level * 0.4f
        }
        .background(colors.error.copy(alpha = 0.3f), CircleShape),
    )
    Box(
      Modifier
        .size(88.dp)
        .clip(CircleShape)
        .background(if (recording) colors.error else colors.primary.copy(alpha = if (sending) 0.4f else 1f))
        .indication(interaction, ripple())
        .pointerInput(sending) {
          if (sending) return@pointerInput
          detectTapGestures(onPress = { offset ->
            val press = PressInteraction.Press(offset)
            interaction.emit(press)
            val wasRecording = isRecording
            if (!wasRecording) {
              haptics.performHapticFeedback(HapticFeedbackType.ToggleOn)
              keys.start()
            }
            val pressedAt = SystemClock.uptimeMillis()
            val released = tryAwaitRelease()
            interaction.emit(if (released) PressInteraction.Release(press) else PressInteraction.Cancel(press))
            if (!released || wasRecording || SystemClock.uptimeMillis() - pressedAt >= HOLD_TO_TALK_MS) {
              haptics.performHapticFeedback(HapticFeedbackType.ToggleOff)
              keys.stop()
            }
          })
        }
        .clearAndSetSemantics {
          role = Role.Button
          contentDescription = if (recording) "Stop dictation" else "Dictate"
          if (!sending) onClick {
            if (recording) keys.stop() else keys.start()
            true
          }
        },
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        if (recording) Icons.Rounded.Stop else Icons.Rounded.Mic,
        contentDescription = null,
        tint = if (recording) colors.onError else colors.onPrimary,
        modifier = Modifier.size(36.dp),
      )
    }
  }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun Key(
  icon: ImageVector,
  description: String,
  modifier: Modifier = Modifier,
  highlighted: Boolean = false,
  onLongClick: (() -> Unit)? = null,
  onClick: () -> Unit,
) {
  val haptics = LocalHapticFeedback.current
  val colors = MaterialTheme.colorScheme
  Box(
    modifier
      .fillMaxHeight()
      .clip(KeyShape)
      .background(if (highlighted) colors.primary else colors.surfaceContainerHighest)
      .combinedClickable(
        onLongClick = onLongClick?.let { longClick ->
          {
            haptics.performHapticFeedback(HapticFeedbackType.LongPress)
            longClick()
          }
        },
        onClick = {
          haptics.performHapticFeedback(HapticFeedbackType.KeyboardTap)
          onClick()
        },
      ),
    contentAlignment = Alignment.Center,
  ) {
    Icon(icon, description, tint = if (highlighted) colors.onPrimary else Color.White)
  }
}

@Composable
private fun BackspaceKey(delete: () -> Unit) {
  val haptics = LocalHapticFeedback.current
  val interaction = remember { MutableInteractionSource() }
  Box(
    Modifier
      .width(64.dp)
      .fillMaxHeight()
      .clip(KeyShape)
      .background(MaterialTheme.colorScheme.surfaceContainerHighest)
      .indication(interaction, ripple())
      .pointerInput(Unit) {
        detectTapGestures(onPress = { offset ->
          val press = PressInteraction.Press(offset)
          interaction.emit(press)
          coroutineScope {
            val repeat = launch {
              var interval = 400L
              while (true) {
                haptics.performHapticFeedback(HapticFeedbackType.KeyboardTap)
                delete()
                delay(interval)
                interval = 50L
              }
            }
            tryAwaitRelease()
            repeat.cancel()
          }
          interaction.emit(PressInteraction.Release(press))
        })
      }
      .clearAndSetSemantics {
        role = Role.Button
        contentDescription = "Delete"
        onClick {
          delete()
          true
        }
      },
    contentAlignment = Alignment.Center,
  ) {
    Icon(Icons.AutoMirrored.Rounded.Backspace, contentDescription = null, tint = Color.White)
  }
}

private fun returnIcon(action: Int?) = when (action) {
  EditorInfo.IME_ACTION_SEARCH -> Icons.Rounded.Search
  EditorInfo.IME_ACTION_SEND -> Icons.AutoMirrored.Rounded.Send
  EditorInfo.IME_ACTION_GO, EditorInfo.IME_ACTION_NEXT -> Icons.AutoMirrored.Rounded.ArrowForward
  EditorInfo.IME_ACTION_PREVIOUS -> Icons.AutoMirrored.Rounded.ArrowBack
  EditorInfo.IME_ACTION_DONE -> Icons.Rounded.Check
  else -> Icons.AutoMirrored.Rounded.KeyboardReturn
}
