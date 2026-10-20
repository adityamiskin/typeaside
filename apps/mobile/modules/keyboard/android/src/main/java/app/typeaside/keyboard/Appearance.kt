package app.typeaside.keyboard

import android.content.Context

data class Appearance(val shape: String = "pill", val size: String = "standard", val position: Float = 0.5f) {
  val height: Int get() = when (size) { "small" -> 32; "large" -> 48; else -> 40 }
  val width: Int get() = if (shape == "circle") height else height + 24

  companion object {
    const val STORE = "typeaside.bubble"
    fun read(context: Context): Appearance {
      val prefs = context.getSharedPreferences(STORE, Context.MODE_PRIVATE)
      return Appearance(prefs.getString("shape", "pill") ?: "pill", prefs.getString("size", "standard") ?: "standard", prefs.getFloat("position", 0.5f))
    }
  }
}
