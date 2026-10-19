package app.typeaside.keyboard

import android.content.Intent
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
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
      context.getSystemService(InputMethodManager::class.java)
        .enabledInputMethodList
        .any { it.packageName == context.packageName }
    }

    AsyncFunction("openSettings") {
      context.startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
  }
}
