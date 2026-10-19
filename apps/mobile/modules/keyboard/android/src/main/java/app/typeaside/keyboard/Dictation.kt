package app.typeaside.keyboard

import android.text.InputType
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID

sealed interface Outcome {
  data class Text(val text: String) : Outcome
  data object Unauthorized : Outcome
  data class Limit(val resetsAt: Long?) : Outcome
  data class Failed(val message: String) : Outcome
}

private const val CURSOR_CONTEXT = 2_000

/** Mirrors contextInput in apps/api/src/dictate.ts, which rejects unknown keys. */
fun dictationContext(editor: EditorInfo, connection: InputConnection?, dictionary: List<String>): JSONObject {
  val variation = editor.inputType and InputType.TYPE_MASK_VARIATION
  val text = editor.inputType and InputType.TYPE_MASK_CLASS == InputType.TYPE_CLASS_TEXT
  val password = (editor.inputType and InputType.TYPE_MASK_CLASS == InputType.TYPE_CLASS_NUMBER &&
    variation == InputType.TYPE_NUMBER_VARIATION_PASSWORD) || (text && variation in setOf(
    InputType.TYPE_TEXT_VARIATION_PASSWORD,
    InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
    InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
  ))
  return JSONObject()
    .put("mode", "dictate")
    .put("category", "personal")
    .putOpt("app_id", editor.packageName)
    .apply {
      if (password || connection == null) return@apply
      connection.getTextBeforeCursor(CURSOR_CONTEXT, 0)?.let { put("before_cursor", it.takeLast(CURSOR_CONTEXT).toString()) }
      connection.getTextAfterCursor(CURSOR_CONTEXT, 0)?.let { put("after_cursor", it.take(CURSOR_CONTEXT).toString()) }
    }
    .put("dictionary", JSONArray(dictionary))
    .put("names", JSONArray())
    .put("tone", "casual")
    .put("translate_to", "English")
}

suspend fun dictate(credentials: Credentials, pcm: ByteArray, context: JSONObject): Outcome = withContext(Dispatchers.IO) {
  val boundary = "typeaside-${UUID.randomUUID()}"
  val body = ByteArrayOutputStream().apply {
    fun part(headers: String, content: ByteArray) {
      write("--$boundary\r\n$headers\r\n\r\n".toByteArray())
      write(content)
      write("\r\n".toByteArray())
    }
    part("Content-Disposition: form-data; name=\"context\"", context.toString().toByteArray())
    part("Content-Disposition: form-data; name=\"audio\"; filename=\"dictation.wav\"\r\nContent-Type: audio/wav", wav(pcm))
    write("--$boundary--\r\n".toByteArray())
  }.toByteArray()

  val connection = URL("${credentials.apiUrl}/v1/dictate").openConnection() as HttpURLConnection
  try {
    connection.requestMethod = "POST"
    connection.doOutput = true
    connection.connectTimeout = 10_000
    connection.readTimeout = 60_000
    connection.setRequestProperty("Authorization", "Bearer ${credentials.token}")
    connection.setRequestProperty("Content-Type", "multipart/form-data; boundary=$boundary")
    // Fixed length sends Content-Length, which the API requires before reading the body.
    connection.setFixedLengthStreamingMode(body.size)
    connection.outputStream.use { it.write(body) }
    val status = connection.responseCode
    val json = (if (status < 400) connection.inputStream else connection.errorStream)
      ?.bufferedReader()
      ?.use { it.readText() }
      ?.let { runCatching { JSONObject(it) }.getOrNull() }
    val error = json?.optJSONObject("error")
    when {
      status in 200..299 && json?.has("text") == true -> Outcome.Text(json.getString("text"))
      status == 401 -> Outcome.Unauthorized
      status == 402 -> Outcome.Limit(error?.optLong("resets_at")?.takeIf { it > 0 })
      else -> Outcome.Failed(error?.optString("message")?.takeIf { it.isNotEmpty() } ?: "Dictation failed ($status)")
    }
  } catch (_: IOException) {
    Outcome.Failed("No connection")
  } finally {
    connection.disconnect()
  }
}
