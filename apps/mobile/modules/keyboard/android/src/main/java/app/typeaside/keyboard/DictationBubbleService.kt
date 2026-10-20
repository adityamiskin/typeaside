package app.typeaside.keyboard

import android.Manifest
import android.accessibilityservice.AccessibilityService
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.graphics.Rect
import android.os.Build
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
  private var appearance by mutableStateOf(Appearance())
  private val preferences by lazy { getSharedPreferences(Appearance.STORE, MODE_PRIVATE) }
  private val appearanceListener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ ->
    appearance = Appearance.read(this)
    shrunk = false
    position()
    scheduleShrink()
  }
  private val handler = Handler(Looper.getMainLooper())
  private var view: ComposeView? = null
  private var fading = false
  private var target: AccessibilityNodeInfo? = null
  private var recorder: Recorder? = null
  private var job: Job? = null
  private var shrunk by mutableStateOf(false)
  private var restoredOnTouch = false
  private var bubbleState by mutableStateOf<Bubble>(Bubble.Idle)
  private var state: Bubble
    get() = bubbleState
    set(value) {
      val wasBusy = bubbleState is Bubble.Recording || bubbleState == Bubble.Sending
      val wasRecording = bubbleState is Bubble.Recording
      bubbleState = value
      if (value != Bubble.Idle) shrunk = false
      if (wasRecording != (value is Bubble.Recording)) position()
      scheduleShrink()
      if (wasBusy && value !is Bubble.Recording && value != Bubble.Sending) refreshOverlay()
    }
  private val params by lazy {
    WindowManager.LayoutParams(
      dp(maxOf(48, appearance.width)), dp(appearance.height + 16),
      WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
      WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL or WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
      PixelFormat.TRANSLUCENT,
    ).apply {
      gravity = Gravity.TOP or Gravity.LEFT
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) setFitInsetsTypes(0)
    }
  }
  private var downX = 0f
  private var originX = 0
  private var dragging = false
  private var held = false
  private var beganRecording = false
  private val shrink = Runnable {
    if (view != null && state == Bubble.Idle && appearance.shrinkWhenIdle) {
      shrunk = true
      position()
    }
  }
  private val hold = Runnable {
    if (!dragging && state !is Bubble.Recording && state != Bubble.Sending) {
      held = true
      start()
    }
  }

  override fun onCreate() {
    super.onCreate()
    appearance = Appearance.read(this)
    preferences.registerOnSharedPreferenceChangeListener(appearanceListener)
    savedState.performRestore(null)
    registry.currentState = Lifecycle.State.RESUMED
    lifecycleScope.launch(AndroidUiDispatcher.Main) { recomposer.runRecomposeAndApplyChanges() }
  }

  override fun onAccessibilityEvent(event: AccessibilityEvent) {
    when (event.eventType) {
      AccessibilityEvent.TYPE_VIEW_FOCUSED,
      AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED,
      AccessibilityEvent.TYPE_WINDOWS_CHANGED,
      AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED -> refreshOverlay(event.eventType == AccessibilityEvent.TYPE_VIEW_FOCUSED)
    }
  }

  private fun refreshOverlay(fieldFocused: Boolean = false) {
    if (state is Bubble.Recording || state == Bubble.Sending) return
    val keyboard = windows.firstOrNull { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD }
    val node = rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
    if (keyboard == null || node == null || !node.isEditable || node.isPassword) {
      hideOverlay(animated = true)
      return
    }
    if (fieldFocused || target != node) {
      shrunk = false
      target = node
      position()
    }
    if (fading) {
      view?.animate()?.cancel()
      view?.alpha = appearance.opacity
      fading = false
    }
    val keyboardBounds = Rect().also(keyboard::getBoundsInScreen)
    params.y = (keyboardBounds.top - params.height).coerceAtLeast(0)
    view?.let { manager.updateViewLayout(it, params) }
    if (view == null) {
      view = ComposeView(this).apply {
        setViewTreeLifecycleOwner(this@DictationBubbleService)
        setViewTreeSavedStateRegistryOwner(this@DictationBubbleService)
        setParentCompositionContext(recomposer)
        setContent { BubbleTheme { MicBubble(state, appearance, shrunk, ::touch, ::toggle, ::cancel) } }
      }.also {
        it.alpha = appearance.opacity
        manager.addView(it, params)
      }
      scheduleShrink()
    } else if (fieldFocused) scheduleShrink()
  }

  private fun touch(event: MotionEvent): Boolean {
    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN -> {
        restoredOnTouch = shrunk
        shrunk = false
        position()
        handler.removeCallbacks(shrink)
        downX = event.rawX
        originX = params.x
        dragging = false
        held = false
        beganRecording = state is Bubble.Recording
        handler.postDelayed(hold, ViewConfiguration.getLongPressTimeout().toLong())
      }
      MotionEvent.ACTION_MOVE -> {
        val dx = event.rawX - downX
        if (!held && abs(dx) > ViewConfiguration.get(this).scaledTouchSlop) {
          dragging = true
          handler.removeCallbacks(hold)
        }
        if (dragging) {
          params.x = (originX + dx.toInt()).coerceIn(0, resources.displayMetrics.widthPixels - params.width)
          view?.let { manager.updateViewLayout(it, params) }
        }
      }
      MotionEvent.ACTION_UP -> {
        handler.removeCallbacks(hold)
        if (dragging) {
          preferences.edit().putFloat("position", (params.x + params.width / 2f) / resources.displayMetrics.widthPixels).apply()
        }
        if (held) recorder?.stop()
        else if (beganRecording && !dragging && event.x < dp(48)) cancel()
        else if (!dragging && !restoredOnTouch) toggle()
        held = false
        scheduleShrink()
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
    if (shrunk) {
      shrunk = false
      position()
      scheduleShrink()
      return
    }
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
    val context = dictationContext(node.field(), credentials.dictionary, credentials.snippets)
    val recording = Recorder().also { recorder = it }
    state = Bubble.Recording(emptyList())
    job = lifecycleScope.launch {
      try {
        val pcm = recording.record { _, level ->
          val levels = (state as? Bubble.Recording)?.levels.orEmpty()
          state = Bubble.Recording((levels + level).takeLast(20))
        }
        recorder = null
        if (!pcm.isLongEnough()) {
          state = Bubble.Idle
          return@launch
        }
        state = Bubble.Sending
        when (val outcome = dictate(credentials, pcm, context)) {
          is Outcome.Text -> {
            HistoryQueue.append(this@DictationBubbleService, outcome.text)
            val clipboard = getSystemService(ClipboardManager::class.java)
            val focused = rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            if (node.refresh() && node.isEditable && !node.isPassword && node == focused) {
              if (!node.insert(outcome.text, clipboard)) error("Could not insert text")
            } else {
              clipboard.setPrimaryClip(ClipData.newPlainText("TypeAside", outcome.text))
              Toast.makeText(this@DictationBubbleService, "Dictation copied to clipboard", Toast.LENGTH_SHORT).show()
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

  private fun hideOverlay(animated: Boolean = false) {
    handler.removeCallbacks(shrink)
    target = null
    val current = view ?: return
    if (animated && fading) return
    current.animate().cancel()
    if (animated) {
      fading = true
      current.animate().alpha(0f).setDuration(120).withEndAction {
        if (view == current && fading) {
          manager.removeView(current)
          current.disposeComposition()
          view = null
          fading = false
        }
      }.start()
    } else {
      manager.removeView(current)
      current.disposeComposition()
      view = null
      fading = false
    }
  }

  private fun hide() {
    cancel()
    hideOverlay()
  }

  override fun onInterrupt() = hide()
  override fun onDestroy() {
    preferences.unregisterOnSharedPreferenceChangeListener(appearanceListener)
    hide()
    recomposer.cancel()
    registry.currentState = Lifecycle.State.DESTROYED
    super.onDestroy()
  }

  private fun position() {
    params.width = dp(if (state is Bubble.Recording) RECORDING_WIDTH_DP else if (shrunk) 48 else maxOf(48, appearance.width))
    params.height = dp(appearance.height + 16)
    params.x = (appearance.position * resources.displayMetrics.widthPixels - params.width / 2).toInt()
      .coerceIn(0, resources.displayMetrics.widthPixels - params.width)
    windows.firstOrNull { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD }?.let { keyboard ->
      val bounds = Rect().also(keyboard::getBoundsInScreen)
      params.y = (bounds.top - params.height).coerceAtLeast(0)
    }
    view?.let {
      it.alpha = appearance.opacity
      manager.updateViewLayout(it, params)
    }
  }

  private fun scheduleShrink() {
    handler.removeCallbacks(shrink)
    if (view != null && state == Bubble.Idle && !shrunk && appearance.shrinkWhenIdle) {
      handler.postDelayed(shrink, 5_000)
    }
  }

  private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
}
