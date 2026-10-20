package app.typeaside.keyboard

import android.content.Context
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import expo.modules.kotlin.AppContext
import expo.modules.kotlin.views.ComposableScope
import expo.modules.kotlin.views.ComposeProps
import expo.modules.kotlin.views.ExpoComposeView

class BubblePreviewProps : ComposeProps

class BubblePreview(context: Context, appContext: AppContext) :
  ExpoComposeView<BubblePreviewProps>(context, appContext, withHostingView = true) {
  override val props = BubblePreviewProps()
  var appearance by mutableStateOf(Appearance())
  var phase by mutableStateOf("idle")

  @Composable
  override fun ComposableScope.Content() {
    val levels = listOf(.18f, .3f, .55f, .75f, .9f, .7f, .95f, .8f, .65f, .8f, .5f, .25f, .1f)
    val state = when (phase) {
      "recording" -> Bubble.Recording(levels)
      "sending" -> Bubble.Sending
      "paused" -> Bubble.Paused(levels)
      "copied" -> Bubble.Success(true)
      "error" -> Bubble.Error("No connection. Tap to try again")
      "limit" -> Bubble.Limit("Resets Monday")
      else -> Bubble.Idle
    }
    Box(Modifier.fillMaxSize().alpha(appearance.opacity), contentAlignment = Alignment.Center) {
        BubbleSurface(state, appearance)
    }
  }
}
