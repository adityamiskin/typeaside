package app.typeaside.keyboard

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
