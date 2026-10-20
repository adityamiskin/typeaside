package app.typeaside.widgets

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import android.content.SharedPreferences
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.ColorFilter
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
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import com.google.android.material.color.utilities.Hct
import com.google.android.material.color.utilities.TonalPalette
import java.text.NumberFormat

private fun open(context: Context, path: String) = actionStartActivity(
  Intent(Intent.ACTION_VIEW, Uri.parse("app.typeaside://$path"))
    .setPackage(context.packageName).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
)

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
  override val sizeMode = SizeMode.Responsive(setOf(DpSize(57.dp, 57.dp), DpSize(180.dp, 57.dp), DpSize(250.dp, 57.dp)))

  override suspend fun provideGlance(context: Context, id: GlanceId) {
    provideContent {
      val snapshot = observeSnapshot(context)
      GlanceTheme {
        val wide = LocalSize.current.width >= 110.dp
        Row(
          GlanceModifier.fillMaxSize().appWidgetBackground()
            .background(GlanceTheme.colors.widgetBackground)
            .cornerRadius(R.dimen.typeaside_widget_outer_radius)
            .clickable(open(context, "dictate?start=1")).padding(12.dp),
          verticalAlignment = Alignment.CenterVertically,
          horizontalAlignment = if (wide) Alignment.Start else Alignment.CenterHorizontally,
        ) {
          Mic(if (wide) 48 else 56, if (wide) 24 else 28)
          if (wide) {
            Spacer(GlanceModifier.width(12.dp))
            Column {
              Text("Dictate", style = TextStyle(color = GlanceTheme.colors.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Medium), maxLines = 1)
              snapshot?.wordsLeft?.let {
                Text("${NumberFormat.getIntegerInstance().format(it)} words left", style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 12.sp), maxLines = 1)
              }
            }
          }
        }
      }
    }
  }
}

@Composable
private fun Mic(circle: Int, glyph: Int) {
  Box(
    GlanceModifier.size(circle.dp).background(GlanceTheme.colors.primaryContainer).cornerRadius((circle / 2).dp),
    contentAlignment = Alignment.Center,
  ) {
    Image(ImageProvider(R.drawable.typeaside_widget_mic), "Dictate", GlanceModifier.size(glyph.dp), colorFilter = ColorFilter.tint(GlanceTheme.colors.onPrimaryContainer))
  }
}

class NotesWidget : GlanceAppWidget() {
  override val sizeMode = SizeMode.Exact

  override suspend fun provideGlance(context: Context, id: GlanceId) {
    provideContent {
      val snapshot = observeSnapshot(context)
      GlanceTheme {
        Column(
          GlanceModifier.fillMaxSize().appWidgetBackground()
            .background(GlanceTheme.colors.widgetBackground)
            .cornerRadius(R.dimen.typeaside_widget_outer_radius)
            .padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 12.dp),
        ) {
          if (snapshot == null) {
            Box(GlanceModifier.fillMaxSize().clickable(open(context, "notes")), contentAlignment = Alignment.Center) {
              Text("Open TypeAside to sign in", style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 14.sp))
            }
          } else {
            Row(GlanceModifier.fillMaxWidth().height(40.dp), verticalAlignment = Alignment.CenterVertically) {
              Text("Notes", GlanceModifier.defaultWeight(), style = TextStyle(color = GlanceTheme.colors.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Medium))
              Box(
                GlanceModifier.size(40.dp).background(GlanceTheme.colors.primaryContainer).cornerRadius(20.dp)
                  .clickable(open(context, "notes/new")), contentAlignment = Alignment.Center,
              ) {
                Image(ImageProvider(R.drawable.typeaside_widget_add), "New note", GlanceModifier.size(22.dp), colorFilter = ColorFilter.tint(GlanceTheme.colors.onPrimaryContainer))
              }
            }
            Spacer(GlanceModifier.height(8.dp))
            if (snapshot.notes.isEmpty()) {
              Box(GlanceModifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No notes yet", style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 14.sp))
              }
            } else {
              val height = LocalSize.current.height.value
              val count = if (height < 180) 2 else if (height < 250) 3 else 4
              LazyColumn {
                items(snapshot.notes.take(count)) { note ->
                  Column {
                    NoteRow(note)
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
}

@Composable
private fun NoteRow(note: WidgetNote) {
  val context = LocalContext.current
  val lines = note.preview.trim().lines()
  val title = note.title.ifBlank { lines.firstOrNull().orEmpty() }
  val preview = if (note.title.isBlank()) lines.drop(1).joinToString(" ") else note.preview
  Row(
    GlanceModifier.fillMaxWidth().background(GlanceTheme.colors.secondaryContainer)
      .cornerRadius(R.dimen.typeaside_widget_inner_radius)
      .clickable(open(context, "notes/${Uri.encode(note.id)}"))
      .padding(horizontal = 12.dp, vertical = 10.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Box(GlanceModifier.size(8.dp).background(noteTint(note.color)).cornerRadius(4.dp)) {}
    Spacer(GlanceModifier.width(8.dp))
    Column {
      Text(title, style = TextStyle(color = GlanceTheme.colors.onSecondaryContainer, fontSize = 14.sp, fontWeight = FontWeight.Medium), maxLines = 1)
      if (preview.isNotBlank()) Text(preview.replace('\n', ' '), style = TextStyle(color = GlanceTheme.colors.onSecondaryContainer, fontSize = 12.sp), maxLines = 1)
    }
  }
}

@Composable
private fun noteTint(key: String): androidx.glance.unit.ColorProvider {
  val seed = when (key) {
    "sky" -> 0xff5f86b3.toInt()
    "lilac" -> 0xff8a6aa3.toInt()
    "sage" -> 0xff5f9472.toInt()
    "sand" -> 0xffa88d55.toInt()
    "rose" -> 0xffa86369.toInt()
    else -> return GlanceTheme.colors.outline
  }
  val hct = Hct.fromInt(seed)
  val palette = TonalPalette.fromHueAndChroma(hct.hue, maxOf(48.0, hct.chroma))
  return ColorProvider(day = Color(palette.tone(40)), night = Color(palette.tone(80)))
}

class DictateWidgetReceiver : GlanceAppWidgetReceiver() {
  override val glanceAppWidget = DictateWidget()
}

class NotesWidgetReceiver : GlanceAppWidgetReceiver() {
  override val glanceAppWidget = NotesWidget()
}
