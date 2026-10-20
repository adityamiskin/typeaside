package app.typeaside.keyboard

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

data class Snippet(val trigger: String, val text: String) {
  fun json(): JSONObject = JSONObject().put("trigger", trigger).put("text", text)
}

data class Credentials(val token: String, val apiUrl: String, val dictionary: List<String>, val snippets: List<Snippet> = emptyList())

// The accessibility service shares the app process and its Keystore-backed credential store.
class CredentialStore(context: Context) {
  private val prefs = context.getSharedPreferences("typeaside.keyboard", Context.MODE_PRIVATE)

  fun read(): Credentials? {
    val sealed = prefs.getString(KEY, null) ?: return null
    return runCatching {
      val bytes = Base64.decode(sealed, Base64.NO_WRAP)
      val cipher = Cipher.getInstance(TRANSFORMATION)
      cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes, 0, IV_SIZE))
      val json = JSONObject(String(cipher.doFinal(bytes, IV_SIZE, bytes.size - IV_SIZE)))
      val words = json.getJSONArray("dictionary")
      val snippets = json.optJSONArray("snippets") ?: JSONArray()
      Credentials(
        token = json.getString("token"),
        apiUrl = json.getString("apiUrl"),
        dictionary = List(words.length(), words::getString),
        snippets = List(snippets.length()) { index -> snippets.getJSONObject(index).let { Snippet(it.getString("trigger"), it.getString("text")) } },
      ).takeIf { it.token.isNotBlank() && it.apiUrl.isNotBlank() }
    }.getOrNull()
  }

  fun write(credentials: Credentials) {
    val json = JSONObject()
      .put("token", credentials.token)
      .put("apiUrl", credentials.apiUrl.trimEnd('/'))
      .put("dictionary", JSONArray(credentials.dictionary))
      .put("snippets", JSONArray(credentials.snippets.map { it.json() }))
    val cipher = Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.ENCRYPT_MODE, key()) }
    val sealed = cipher.iv + cipher.doFinal(json.toString().toByteArray())
    prefs.edit().putString(KEY, Base64.encodeToString(sealed, Base64.NO_WRAP)).apply()
  }

  fun clear() {
    prefs.edit().remove(KEY).apply()
  }

  private fun key(): SecretKey {
    val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    (keyStore.getKey(ALIAS, null) as? SecretKey)?.let { return it }
    val spec = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
      .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
      .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
      .build()
    return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
      .apply { init(spec) }
      .generateKey()
  }

  private companion object {
    const val KEY = "credentials"
    const val ALIAS = "typeaside.keyboard"
    const val TRANSFORMATION = "AES/GCM/NoPadding"
    const val IV_SIZE = 12
  }
}
