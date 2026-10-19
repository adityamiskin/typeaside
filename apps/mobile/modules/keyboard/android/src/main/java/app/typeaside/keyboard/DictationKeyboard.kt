package app.typeaside.keyboard

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.inputmethodservice.InputMethodService
import android.net.Uri
import android.os.Build
import android.text.format.DateUtils
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.ComposeView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.savedstate.SavedStateRegistryController
import androidx.savedstate.SavedStateRegistryOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

class DictationKeyboard : InputMethodService(), LifecycleOwner, SavedStateRegistryOwner, Keys {
  private val lifecycleRegistry = LifecycleRegistry(this)
  private val savedStateController = SavedStateRegistryController.create(this)
  override val lifecycle: Lifecycle get() = lifecycleRegistry
  override val savedStateRegistry get() = savedStateController.savedStateRegistry

  private val store by lazy { CredentialStore(this) }
  private var recorder: Recorder? = null
  private var dictation: Job? = null
  private var panel by mutableStateOf<Panel>(Panel.Ready())
  private var returnAction by mutableStateOf<Int?>(null)

  override fun onCreate() {
    super.onCreate()
    savedStateController.performRestore(null)
    lifecycleRegistry.currentState = Lifecycle.State.RESUMED
  }

  override fun onDestroy() {
    cancelDictation()
    super.onDestroy()
    lifecycleRegistry.currentState = Lifecycle.State.DESTROYED
  }

  override fun onCreateInputView(): View {
    // Compose looks these owners up from the window's root view, which an IME window does not set.
    window.window?.decorView?.let {
      it.setViewTreeLifecycleOwner(this)
      it.setViewTreeSavedStateRegistryOwner(this)
    }
    return ComposeView(this).apply {
      setContent { KeyboardTheme { KeyboardPanel(panel, returnAction, this@DictationKeyboard) } }
    }
  }

  override fun onEvaluateFullscreenMode() = false

  override fun onStartInput(editor: EditorInfo, restarting: Boolean) {
    cancelDictation()
    super.onStartInput(editor, restarting)
  }

  override fun onFinishInput() {
    cancelDictation()
    super.onFinishInput()
  }

  override fun onStartInputView(editor: EditorInfo, restarting: Boolean) {
    super.onStartInputView(editor, restarting)
    val action = editor.imeOptions and EditorInfo.IME_MASK_ACTION
    returnAction = action.takeIf {
      editor.imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION == 0 &&
        it != EditorInfo.IME_ACTION_NONE &&
        it != EditorInfo.IME_ACTION_UNSPECIFIED
    }
    if (dictation?.isActive != true) panel = idle()
  }

  override fun onFinishInputView(finishingInput: Boolean) {
    cancelDictation()
    super.onFinishInputView(finishingInput)
  }

  private fun cancelDictation() {
    recorder?.stop()
    dictation?.cancel()
    recorder = null
    dictation = null
  }

  private fun idle() = when {
    store.read() == null -> Panel.SignedOut
    checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED -> Panel.NoMicrophone
    else -> Panel.Ready()
  }

  override fun start() {
    if (dictation?.isActive == true) return
    val credentials = store.read() ?: return run { panel = Panel.SignedOut }
    if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
      panel = Panel.NoMicrophone
      return
    }
    val editor = currentInputEditorInfo ?: return
    val connection = currentInputConnection ?: return
    val context = dictationContext(editor, connection, credentials.dictionary)
    val recording = Recorder().also { recorder = it }
    panel = Panel.Recording(seconds = 0, level = 0f)
    dictation = lifecycleScope.launch {
      val pcm = try {
        recording.record { seconds, level -> panel = Panel.Recording(seconds, level) }
      } catch (_: IllegalStateException) {
        null
      } catch (_: SecurityException) {
        null
      }
      if (pcm == null) {
        panel = Panel.Ready("Microphone unavailable")
        return@launch
      }
      if (!pcm.isLongEnough()) {
        panel = Panel.Ready()
        return@launch
      }
      panel = Panel.Sending
      panel = when (val outcome = dictate(credentials, pcm, context)) {
        is Outcome.Text -> {
          if (outcome.text.isNotEmpty()) connection.commitText(outcome.text, 1)
          Panel.Ready()
        }
        Outcome.Unauthorized -> {
          store.clear()
          Panel.SignedOut
        }
        is Outcome.Limit -> Panel.Ready(outcome.resetsAt?.let { "Weekly limit reached. Resets ${resetDate(it)}" } ?: "Weekly limit reached")
        is Outcome.Failed -> Panel.Ready(outcome.message)
      }
    }
  }

  override fun stop() {
    recorder?.stop()
  }

  private fun resetDate(millis: Long) = DateUtils.formatDateTime(
    this,
    millis,
    DateUtils.FORMAT_SHOW_WEEKDAY or DateUtils.FORMAT_ABBREV_WEEKDAY or DateUtils.FORMAT_SHOW_DATE or DateUtils.FORMAT_ABBREV_MONTH,
  )

  override fun openApp() {
    startActivity(
      Intent(Intent.ACTION_VIEW, Uri.parse("app.typeaside://"))
        .setPackage(packageName)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
    )
  }

  override fun switchKeyboard() {
    val switched = Build.VERSION.SDK_INT >= Build.VERSION_CODES.P && switchToNextInputMethod(false)
    if (!switched) pickKeyboard()
  }

  override fun pickKeyboard() {
    getSystemService(InputMethodManager::class.java).showInputMethodPicker()
  }

  override fun delete() = sendDownUpKeyEvents(KeyEvent.KEYCODE_DEL)

  override fun space() {
    currentInputConnection?.commitText(" ", 1)
  }

  override fun enter() {
    val action = returnAction
    if (action == null) sendDownUpKeyEvents(KeyEvent.KEYCODE_ENTER)
    else currentInputConnection?.performEditorAction(action)
  }
}
