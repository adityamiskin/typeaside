package app.typeaside.keyboard

import android.content.Context
import kotlin.math.roundToInt

data class Appearance(
  val shape: String = "pill",
  val size: Float = 0.5f,
  val opacity: Float = 1f,
  val shrinkWhenIdle: Boolean = false,
  val position: Float = 0.5f,
) {
  val height: Int get() = (32 + size * 24).roundToInt()
  val width: Int get() = if (shape == "circle") height else height + 24

  companion object {
    const val STORE = "typeaside.bubble"
    fun read(context: Context): Appearance {
      val prefs = context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
      if (prefs.all["size"] is String) return Appearance()
      return Appearance(
        shape = prefs.getString("shape", "pill") ?: "pill",
        size = prefs.getFloat("size", 0.5f).coerceIn(0f, 1f),
        opacity = prefs.getFloat("opacity", 1f).coerceIn(0.2f, 1f),
        shrinkWhenIdle = prefs.getBoolean("shrinkWhenIdle", false),
        position = prefs.getFloat("position", 0.5f).coerceIn(0f, 1f),
      )
    }
  }
}
