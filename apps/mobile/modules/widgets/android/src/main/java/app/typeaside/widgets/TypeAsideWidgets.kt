package app.typeaside.widgets

import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.os.Build
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.ColorFilter
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.LocalContext
import androidx.glance.LocalSize
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.appWidgetBackground
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.lazy.LazyColumn
import androidx.glance.appwidget.lazy.items
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.color.ColorProvider
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.layout.width
import androidx.glance.material3.ColorProviders
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import com.google.android.material.color.utilities.Hct
import com.google.android.material.color.utilities.MaterialDynamicColors
import com.google.android.material.color.utilities.SchemeTonalSpot
import java.text.NumberFormat

private fun open(context: Context, path: String) = actionStartActivity(
  Intent(Intent.ACTION_VIEW, Uri.parse("app.typeaside://$path"))
    .setPackage(context.packageName).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
)

@Composable
private fun WidgetTheme(content: @Composable () -> Unit) {
  val context = LocalContext.current
  val colors = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
    ColorProviders(light = dynamicLightColorScheme(context), dark = dynamicDarkColorScheme(context))
  } else {
    ColorProviders(light = lightColorScheme(), dark = darkColorScheme())
  }
  GlanceTheme(colors = colors, content = content)
}

@Composable
private fun observeSnapshot(context: Context): WidgetSnapshot? {
  val snapshot by produceState(WidgetSnapshot.read(context), context) {
    val preferences = context.getSharedPreferences(SNAPSHOT_STORE, 0)
    val listener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ ->
      value = WidgetSnapshot.read(context)
    }
    preferences.registerOnSharedPreferenceChangeListener(listener)
    value = WidgetSnapshot.read(context)
    awaitDispose { preferences.unregisterOnSharedPreferenceChangeListener(listener) }
  }
  return snapshot
}

class DictateWidget : GlanceAppWidget() {
  override val sizeMode = SizeMode.Responsive(setOf(DpSize(57.dp, 57.dp), DpSize(130.dp, 57.dp), DpSize(250.dp, 57.dp)))

  override suspend fun provideGlance(context: Context, id: GlanceId) {
    provideContent {
      val snapshot = observeSnapshot(context)
      WidgetTheme {
        val width = LocalSize.current.width
        val wide = width >= 110.dp
        Row(
          GlanceModifier.fillMaxSize().appWidgetBackground()
            .background(GlanceTheme.colors.primaryContainer)
            .cornerRadius(R.dimen.typeaside_widget_outer_radius)
            .clickable(open(context, "dictate?start=1"))
            .padding(start = if (wide) 20.dp else 0.dp, end = if (width >= 250.dp) 20.dp else 0.dp),
          verticalAlignment = Alignment.CenterVertically,
          horizontalAlignment = if (wide) Alignment.Start else Alignment.CenterHorizontally,
        ) {
          Image(ImageProvider(R.drawable.typeaside_widget_mic), "Dictate", GlanceModifier.size(if (wide) 28.dp else 32.dp), colorFilter = ColorFilter.tint(GlanceTheme.colors.onPrimaryContainer))
          if (wide) {
            Spacer(GlanceModifier.width(12.dp))
            Text("Dictate", style = TextStyle(color = GlanceTheme.colors.onPrimaryContainer, fontSize = 18.sp, fontWeight = FontWeight.Medium), maxLines = 1)
            if (width >= 250.dp && snapshot?.wordsLeft != null) {
              Spacer(GlanceModifier.defaultWeight())
              Text("${NumberFormat.getIntegerInstance().format(snapshot.wordsLeft)} left", style = TextStyle(color = GlanceTheme.colors.onPrimaryContainer, fontSize = 13.sp), maxLines = 1)
            }
          }
        }
      }
    }
  }
}

class NotesWidget : GlanceAppWidget() {
  override val sizeMode = SizeMode.Exact

  override suspend fun provideGlance(context: Context, id: GlanceId) {
    provideContent {
      val snapshot = observeSnapshot(context)
      WidgetTheme {
        Column(
          GlanceModifier.fillMaxSize().appWidgetBackground()
            .background(GlanceTheme.colors.widgetBackground)
            .cornerRadius(R.dimen.typeaside_widget_outer_radius).padding(12.dp),
        ) {
          Row(GlanceModifier.fillMaxWidth().height(48.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Notes", GlanceModifier.defaultWeight().padding(start = 4.dp), style = TextStyle(color = GlanceTheme.colors.onSurface, fontSize = 18.sp, fontWeight = FontWeight.Medium), maxLines = 1)
            CaptureButton(R.drawable.typeaside_widget_mic, "Dictate", "dictate?start=1", primary = false)
            Spacer(GlanceModifier.width(8.dp))
            CaptureButton(R.drawable.typeaside_widget_add, "New note", "notes/new", primary = true)
          }
          Spacer(GlanceModifier.height(8.dp))
          if (snapshot == null || snapshot.notes.isEmpty()) {
            EmptyNotes(signedIn = snapshot != null)
          } else {
            val height = LocalSize.current.height.value
            LazyColumn {
              items(snapshot.notes.take(6)) { note ->
                Column {
                  NoteRow(note, previewLines = if (height >= 220) 2 else 1)
                  Spacer(GlanceModifier.height(6.dp))
                }
              }
            }
          }
        }
      }
    }
  }
}

@Composable
private fun CaptureButton(icon: Int, description: String, path: String, primary: Boolean) {
  Box(
    GlanceModifier.size(40.dp)
      .background(if (primary) GlanceTheme.colors.primaryContainer else GlanceTheme.colors.secondaryContainer)
      .cornerRadius(20.dp).clickable(open(LocalContext.current, path)),
    contentAlignment = Alignment.Center,
  ) {
    Image(ImageProvider(icon), description, GlanceModifier.size(22.dp), colorFilter = ColorFilter.tint(if (primary) GlanceTheme.colors.onPrimaryContainer else GlanceTheme.colors.onSecondaryContainer))
  }
}

@Composable
private fun EmptyNotes(signedIn: Boolean) {
  val context = LocalContext.current
  Column(GlanceModifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally, verticalAlignment = Alignment.CenterVertically) {
    Image(ImageProvider(R.drawable.typeaside_widget_note), null, GlanceModifier.size(36.dp), colorFilter = ColorFilter.tint(GlanceTheme.colors.onSurfaceVariant))
    Spacer(GlanceModifier.height(8.dp))
    Text(if (signedIn) "No notes yet" else "Sign in to see your notes", style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 14.sp))
    Spacer(GlanceModifier.height(12.dp))
    Box(
      GlanceModifier.height(40.dp).background(GlanceTheme.colors.secondaryContainer).cornerRadius(20.dp)
        .clickable(open(context, if (signedIn) "notes/new" else "notes")).padding(horizontal = 20.dp),
      contentAlignment = Alignment.Center,
    ) {
      Text(if (signedIn) "New note" else "Open TypeAside", style = TextStyle(color = GlanceTheme.colors.onSecondaryContainer, fontSize = 14.sp, fontWeight = FontWeight.Medium))
    }
  }
}

@Composable
private fun NoteRow(note: WidgetNote, previewLines: Int) {
  val context = LocalContext.current
  val lines = note.preview.trim().lines()
  val title = note.title.ifBlank { lines.firstOrNull().orEmpty() }
  val preview = if (note.title.isBlank()) lines.drop(1).joinToString(" ") else note.preview
  Column(
    GlanceModifier.fillMaxWidth().background(noteContainer(note.color))
      .cornerRadius(R.dimen.typeaside_widget_inner_radius)
      .clickable(open(context, "notes/${Uri.encode(note.id)}"))
      .padding(horizontal = 14.dp, vertical = 12.dp),
  ) {
    Text(title, style = TextStyle(color = GlanceTheme.colors.onSurface, fontSize = 15.sp, fontWeight = FontWeight.Medium), maxLines = 1)
    if (preview.isNotBlank()) Text(preview.replace('\n', ' '), style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 13.sp), maxLines = previewLines)
  }
}

@Composable
private fun noteContainer(key: String): androidx.glance.unit.ColorProvider {
  val seed = when (key) {
    "sky" -> 0xff5f86b3.toInt()
    "lilac" -> 0xff8a6aa3.toInt()
    "sage" -> 0xff5f9472.toInt()
    "sand" -> 0xffa88d55.toInt()
    "rose" -> 0xffa86369.toInt()
    else -> return GlanceTheme.colors.surfaceVariant
  }
  val hct = Hct.fromInt(seed)
  val container = MaterialDynamicColors().primaryContainer()
  return ColorProvider(
    day = Color(container.getArgb(SchemeTonalSpot(hct, false, 0.0))),
    night = Color(container.getArgb(SchemeTonalSpot(hct, true, 0.0))),
  )
}

class DictateWidgetReceiver : GlanceAppWidgetReceiver() {
  override val glanceAppWidget = DictateWidget()
}

class NotesWidgetReceiver : GlanceAppWidgetReceiver() {
  override val glanceAppWidget = NotesWidget()
}
