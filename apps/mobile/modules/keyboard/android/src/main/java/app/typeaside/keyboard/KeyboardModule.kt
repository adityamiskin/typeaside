package app.typeaside.keyboard

import android.content.Context
import android.content.Intent
import android.provider.Settings
import android.content.ComponentName
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import expo.modules.kotlin.records.Field
import expo.modules.kotlin.records.Record

class SnippetRecord : Record {
  @Field val trigger: String = ""
  @Field val text: String = ""
}

class CredentialsRecord : Record {
  @Field val token: String = ""
  @Field val apiUrl: String = ""
  @Field val dictionary: List<String> = emptyList()
  @Field val snippets: List<SnippetRecord> = emptyList()
}

class AppearanceRecord : Record {
  @Field val shape: String = "pill"
  @Field val size: Float = 0.5f
  @Field val opacity: Float = 1f
  @Field val shrinkWhenIdle: Boolean = false
  @Field val position: Float = 0.5f
}

class KeyboardModule : Module() {
  private val context get() = requireNotNull(appContext.reactContext)

  override fun definition() = ModuleDefinition {
    Name("Keyboard")

    AsyncFunction("setCredentials") { credentials: CredentialsRecord ->
      CredentialStore(context).write(Credentials(credentials.token, credentials.apiUrl, credentials.dictionary, credentials.snippets.map { Snippet(it.trigger, it.text) }))
    }

    AsyncFunction("clearCredentials") {
      CredentialStore(context).clear()
    }

    AsyncFunction("getAppearance") {
      val appearance = Appearance.read(context)
      mapOf("shape" to appearance.shape, "size" to appearance.size, "opacity" to appearance.opacity, "shrinkWhenIdle" to appearance.shrinkWhenIdle, "position" to appearance.position)
    }

    AsyncFunction("setAppearance") { appearance: AppearanceRecord ->
      require(appearance.shape in listOf("pill", "circle"))
      require(appearance.size.isFinite() && appearance.size in 0f..1f)
      require(appearance.opacity.isFinite() && appearance.opacity in 0.2f..1f)
      require(appearance.position.isFinite() && appearance.position in 0f..1f)
      context.getSharedPreferences(Appearance.STORE, Context.MODE_PRIVATE).edit()
        .putString("shape", appearance.shape).putFloat("size", appearance.size)
        .putFloat("opacity", appearance.opacity).putBoolean("shrinkWhenIdle", appearance.shrinkWhenIdle)
        .putFloat("position", appearance.position).apply()
    }

    AsyncFunction("isEnabled") {
      val component = ComponentName(context, DictationBubbleService::class.java)
      Settings.Secure.getString(context.contentResolver, Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES)
        ?.split(':')
        ?.any { ComponentName.unflattenFromString(it) == component } == true
    }

    AsyncFunction("openSettings") {
      context.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
  }
}
