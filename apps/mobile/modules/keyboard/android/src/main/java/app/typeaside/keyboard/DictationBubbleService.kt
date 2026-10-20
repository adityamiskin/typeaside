package app.typeaside.keyboard

import android.Manifest
import android.accessibilityservice.AccessibilityService
import android.content.ClipboardManager
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.graphics.Rect
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.MotionEvent
import android.view.ViewConfiguration
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import android.widget.Toast
import androidx.compose.runtime.Recomposer
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.AndroidUiDispatcher
import androidx.compose.ui.platform.ComposeView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.savedstate.SavedStateRegistryController
import androidx.savedstate.SavedStateRegistryOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.abs

class DictationBubbleService : AccessibilityService(), LifecycleOwner, SavedStateRegistryOwner {
  private val registry = LifecycleRegistry(this)
  private val savedState = SavedStateRegistryController.create(this)
  private val recomposer by lazy { Recomposer(AndroidUiDispatcher.Main) }
  override val lifecycle: Lifecycle get() = registry
  override val savedStateRegistry get() = savedState.savedStateRegistry
  private val manager by lazy { getSystemService(WindowManager::class.java) }
  private val store by lazy { CredentialStore(this) }
  private val handler = Handler(Looper.getMainLooper())
  private var view: ComposeView? = null
  private var target: AccessibilityNodeInfo? = null
  private var recorder: Recorder? = null
  private var job: Job? = null
  private var state by mutableStateOf<Bubble>(Bubble.Idle)
  private val params by lazy {
    WindowManager.LayoutParams(
      dp(BUBBLE_WINDOW_DP), dp(BUBBLE_WINDOW_DP),
      WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
      WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
      PixelFormat.TRANSLUCENT,
    ).apply { gravity = Gravity.TOP or Gravity.LEFT }
  }
  private var downX = 0f
  private var downY = 0f
  private var originX = 0
  private var originY = 0
  private var dragging = false
  private var held = false
  private var beganRecording = false
  private val hold = Runnable {
    if (!dragging && state !is Bubble.Recording && state != Bubble.Sending) {
      held = true
      start()
    }
  }

  override fun onCreate() {
    super.onCreate()
    savedState.performRestore(null)
    registry.currentState = Lifecycle.State.RESUMED
    lifecycleScope.launch(AndroidUiDispatcher.Main) { recomposer.runRecomposeAndApplyChanges() }
  }

  override fun onAccessibilityEvent(event: AccessibilityEvent) {
    val keyboard = windows.any { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD }
    val node = rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
    if (!keyboard || node == null || !node.isEditable || node.isPassword || node.packageName == packageName) {
      hide()
      return
    }
    if (target != node) {
      cancel()
      target = node
      val bounds = Rect().also(node::getBoundsInScreen)
      params.x = resources.displayMetrics.widthPixels - params.width - dp(8)
      params.y = (bounds.bottom - dp(BUBBLE_WINDOW_DP)).coerceAtLeast(dp(32))
    }
    if (view == null) {
      view = ComposeView(this).apply {
        setViewTreeLifecycleOwner(this@DictationBubbleService)
        setViewTreeSavedStateRegistryOwner(this@DictationBubbleService)
        setParentCompositionContext(recomposer)
        setContent { BubbleTheme { MicBubble(state, ::touch, ::toggle) } }
      }.also { manager.addView(it, params) }
    }
  }

  private fun touch(event: MotionEvent): Boolean {
    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN -> {
        downX = event.rawX
        downY = event.rawY
        originX = params.x
        originY = params.y
        dragging = false
        held = false
        beganRecording = state is Bubble.Recording
        handler.postDelayed(hold, ViewConfiguration.getLongPressTimeout().toLong())
      }
      MotionEvent.ACTION_MOVE -> {
        val dx = event.rawX - downX
        val dy = event.rawY - downY
        if (!held && (abs(dx) > ViewConfiguration.get(this).scaledTouchSlop || abs(dy) > ViewConfiguration.get(this).scaledTouchSlop)) {
          dragging = true
          handler.removeCallbacks(hold)
        }
        if (dragging) {
          params.x = (originX + dx.toInt()).coerceIn(0, resources.displayMetrics.widthPixels - params.width)
          params.y = (originY + dy.toInt()).coerceIn(0, resources.displayMetrics.heightPixels - params.height)
          view?.let { manager.updateViewLayout(it, params) }
        }
      }
      MotionEvent.ACTION_UP -> {
        handler.removeCallbacks(hold)
        if (held || (beganRecording && !dragging)) recorder?.stop()
        else if (!dragging) toggle()
        held = false
      }
      MotionEvent.ACTION_CANCEL -> {
        handler.removeCallbacks(hold)
        if (held) cancel()
        held = false
      }
    }
    return true
  }

  private fun toggle() {
    when (state) {
      is Bubble.Recording -> recorder?.stop()
      Bubble.Sending -> Unit
      else -> start()
    }
  }

  private fun start() {
    if (job?.isActive == true) return
    val credentials = store.read()
    if (credentials == null || checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
      packageManager.getLaunchIntentForPackage(packageName)?.let { startActivity(it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
      return
    }
    val node = target ?: return
    if (!node.refresh() || !node.isFocused || node.isPassword) return
    val context = dictationContext(node.field(), credentials.dictionary)
    val recording = Recorder().also { recorder = it }
    state = Bubble.Recording(0f)
    job = lifecycleScope.launch {
      try {
        val pcm = recording.record { _, level -> state = Bubble.Recording(level) }
        recorder = null
        if (!pcm.isLongEnough()) {
          state = Bubble.Idle
          return@launch
        }
        state = Bubble.Sending
        when (val outcome = dictate(credentials, pcm, context)) {
          is Outcome.Text -> {
            if (node.refresh() && node.isFocused && !node.isPassword && node == target) {
              if (!node.insert(outcome.text, getSystemService(ClipboardManager::class.java))) error("Could not insert text")
            }
            state = Bubble.Idle
          }
          Outcome.Unauthorized -> {
            store.clear()
            state = Bubble.Error
            Toast.makeText(this@DictationBubbleService, "Open TypeAside to sign in", Toast.LENGTH_SHORT).show()
          }
          is Outcome.Limit -> error("Weekly limit reached")
          is Outcome.Failed -> error(outcome.message)
        }
      } catch (cancelled: CancellationException) {
        throw cancelled
      } catch (error: Exception) {
        state = Bubble.Error
        Toast.makeText(this@DictationBubbleService, error.message ?: "Dictation failed", Toast.LENGTH_SHORT).show()
      }
    }
  }

  private fun cancel() {
    handler.removeCallbacks(hold)
    recorder?.stop()
    job?.cancel()
    recorder = null
    job = null
    state = Bubble.Idle
  }

  private fun hide() {
    cancel()
    target = null
    view?.let { manager.removeView(it); it.disposeComposition() }
    view = null
  }

  override fun onInterrupt() = hide()
  override fun onDestroy() {
    hide()
    recomposer.cancel()
    registry.currentState = Lifecycle.State.DESTROYED
    super.onDestroy()
  }

  private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
}
