package app.typeaside.widgets

import androidx.glance.appwidget.updateAll
import expo.modules.kotlin.functions.Coroutine
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition

class WidgetsModule : Module() {
  override fun definition() = ModuleDefinition {
    Name("Widgets")

    AsyncFunction("setSnapshot") Coroutine { json: String ->
      val context = requireNotNull(appContext.reactContext)
      val snapshot = WidgetSnapshot.parse(json)
      context.getSharedPreferences(SNAPSHOT_STORE, 0).edit()
        .putString(SNAPSHOT_KEY, snapshot.toJson()).commit()
      DictateWidget().updateAll(context)
      NotesWidget().updateAll(context)
    }

    AsyncFunction("clear") Coroutine { ->
      val context = requireNotNull(appContext.reactContext)
      context.getSharedPreferences(SNAPSHOT_STORE, 0).edit().clear().commit()
      DictateWidget().updateAll(context)
      NotesWidget().updateAll(context)
    }
  }
}
