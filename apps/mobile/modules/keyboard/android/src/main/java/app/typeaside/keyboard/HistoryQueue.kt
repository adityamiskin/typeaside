package app.typeaside.keyboard

import android.content.Context
import android.util.AtomicFile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID

// Acknowledge only after JS has persisted the merged history. Replays dedupe by ID.
object HistoryQueue {
  var onChanged: (() -> Unit)? = null

  private fun file(context: Context) = AtomicFile(File(context.filesDir, "bubble-history.json"))

  @Synchronized
  fun read(context: Context): JSONArray {
    val file = file(context)
    return if (file.baseFile.exists()) JSONArray(String(file.readFully(), Charsets.UTF_8)) else JSONArray()
  }

  private fun write(context: Context, entries: JSONArray) {
    val file = file(context)
    val stream = file.startWrite()
    try {
      stream.write(entries.toString().toByteArray(Charsets.UTF_8))
      file.finishWrite(stream)
    } catch (error: Exception) {
      file.failWrite(stream)
      throw error
    }
  }

  @Synchronized
  fun append(context: Context, text: String) {
    val entries = read(context)
    entries.put(JSONObject().put("id", UUID.randomUUID().toString()).put("createdAt", System.currentTimeMillis()).put("text", text))
    write(context, entries)
    onChanged?.invoke()
  }

  @Synchronized
  fun acknowledge(context: Context, ids: List<String>) {
    val consumed = ids.toSet()
    val entries = read(context)
    val remaining = JSONArray()
    for (index in 0 until entries.length()) {
      val entry = entries.getJSONObject(index)
      if (entry.getString("id") !in consumed) remaining.put(entry)
    }
    write(context, remaining)
  }
}
