package app.typeaside.keyboard

import android.content.ClipData
import android.content.ClipboardManager
import android.os.Build
import android.os.Bundle
import android.view.accessibility.AccessibilityNodeInfo

data class Field(val packageName: String?, val text: String, val start: Int, val end: Int)

fun AccessibilityNodeInfo.field(): Field {
  val text = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && isShowingHintText) "" else text?.toString().orEmpty()
  val bounds = 0..text.length
  val (start, end) = if (textSelectionStart in bounds && textSelectionEnd in bounds) {
    minOf(textSelectionStart, textSelectionEnd) to maxOf(textSelectionStart, textSelectionEnd)
  } else {
    text.length to text.length
  }
  return Field(packageName?.toString(), text, start, end)
}

/** Replaces the selection with [transcript]; false when the app accepts neither SET_TEXT nor a paste. */
fun AccessibilityNodeInfo.insert(transcript: String, clipboard: ClipboardManager): Boolean {
  refresh()
  val field = field()
  val before = field.text.take(field.start)
  val spaced = if (before.isNotEmpty() && !before.last().isWhitespace()) " $transcript" else transcript
  val text = before + spaced + field.text.drop(field.end)
  if (performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, Bundle().apply {
      putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text)
    })) {
    val cursor = field.start + spaced.length
    performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION, Bundle().apply {
      putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT, cursor)
      putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT, cursor)
    })
    return true
  }
  // The target reads the clipboard while handling ACTION_PASTE, so the user's clip can go back right after.
  val previous = clipboard.primaryClip
  clipboard.setPrimaryClip(ClipData.newPlainText("TypeAside", spaced))
  val pasted = performAction(AccessibilityNodeInfo.ACTION_PASTE)
  if (previous != null) clipboard.setPrimaryClip(previous)
  else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) clipboard.clearPrimaryClip()
  return pasted
}
