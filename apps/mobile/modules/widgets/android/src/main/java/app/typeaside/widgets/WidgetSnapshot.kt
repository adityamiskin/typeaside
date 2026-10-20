package app.typeaside.widgets

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

internal const val SNAPSHOT_STORE = "typeaside.widgets"
internal const val SNAPSHOT_KEY = "snapshot"

internal data class WidgetNote(
  val id: String,
  val title: String,
  val preview: String,
  val color: String,
  val updatedAt: Long,
)

internal data class WidgetSnapshot(val wordsLeft: Long?, val notes: List<WidgetNote>) {
  fun toJson(): String = JSONObject().apply {
    put("wordsLeft", wordsLeft ?: JSONObject.NULL)
    put("notes", JSONArray().apply {
      notes.forEach { note -> put(JSONObject().apply {
        put("id", note.id)
        put("title", note.title)
        put("preview", note.preview)
        put("color", note.color)
        put("updatedAt", note.updatedAt)
      }) }
    })
  }.toString()

  companion object {
    fun read(context: Context): WidgetSnapshot? =
      context.getSharedPreferences(SNAPSHOT_STORE, 0).getString(SNAPSHOT_KEY, null)
        ?.let { runCatching { parse(it) }.getOrNull() }

    fun parse(json: String): WidgetSnapshot {
      val value = JSONObject(json)
      val rows = value.getJSONArray("notes")
      return WidgetSnapshot(
        if (value.isNull("wordsLeft")) null else value.getLong("wordsLeft").coerceAtLeast(0),
        (0 until rows.length()).map { index ->
          val note = rows.getJSONObject(index)
          WidgetNote(
            note.getString("id"), note.getString("title").take(200),
            note.getString("preview").take(500), note.getString("color"), note.getLong("updatedAt"),
          )
        }.sortedByDescending { it.updatedAt }.take(6),
      )
    }
  }
}
