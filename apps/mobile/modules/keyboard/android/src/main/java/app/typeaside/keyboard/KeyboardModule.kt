package app.typeaside.keyboard

import android.content.Context
import android.content.Intent
import android.provider.Settings
import android.content.ComponentName
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import expo.modules.kotlin.records.Field
import expo.modules.kotlin.records.Record

class CredentialsRecord : Record {
  @Field val token: String = ""
  @Field val apiUrl: String = ""
  @Field val dictionary: List<String> = emptyList()
}

class AppearanceRecord : Record {
  @Field val shape: String = "pill"
  @Field val size: String = "standard"
  @Field val position: Float = 0.5f
}

class KeyboardModule : Module() {
  private val context get() = requireNotNull(appContext.reactContext)

  override fun definition() = ModuleDefinition {
    Name("Keyboard")

    AsyncFunction("setCredentials") { credentials: CredentialsRecord ->
      CredentialStore(context).write(Credentials(credentials.token, credentials.apiUrl, credentials.dictionary))
    }

    AsyncFunction("clearCredentials") {
      CredentialStore(context).clear()
    }

    AsyncFunction("getAppearance") {
      val appearance = Appearance.read(context)
      mapOf("shape" to appearance.shape, "size" to appearance.size, "position" to appearance.position)
    }

    AsyncFunction("setAppearance") { appearance: AppearanceRecord ->
      require(appearance.shape in listOf("pill", "circle"))
      require(appearance.size in listOf("small", "standard", "large"))
      require(appearance.position.isFinite() && appearance.position in 0f..1f)
      context.getSharedPreferences(Appearance.STORE, Context.MODE_PRIVATE).edit()
        .putString("shape", appearance.shape).putString("size", appearance.size)
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
