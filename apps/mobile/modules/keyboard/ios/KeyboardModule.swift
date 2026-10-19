import ExpoModulesCore
import UIKit

struct CredentialsRecord: Record {
  @Field var token = ""
  @Field var apiUrl = ""
  @Field var dictionary: [String] = []
}

public final class KeyboardModule: Module {
  public func definition() -> ModuleDefinition {
    Name("Keyboard")

    OnCreate {
      Task { @MainActor in KeyboardSession.shared.activate() }
    }

    AsyncFunction("setCredentials") { (credentials: CredentialsRecord) in
      try Credentials(
        token: credentials.token, apiUrl: credentials.apiUrl, dictionary: credentials.dictionary
      ).save()
    }

    AsyncFunction("clearCredentials") {
      Credentials.clear()
      await KeyboardSession.shared.end()
    }

    // Best effort: UIKit exposes no public API for enabled keyboards.
    AsyncFunction("isEnabled") { () -> Bool in
      let identifier = NSSelectorFromString("identifier")
      return MainActor.assumeIsolated {
        UITextInputMode.activeInputModes.contains {
          $0.responds(to: identifier)
            && $0.value(forKey: "identifier") as? String == "app.typeaside.keyboard"
        }
      }
    }.runOnQueue(.main)

    AsyncFunction("openSettings") {
      _ = await UIApplication.shared.open(URL(string: UIApplication.openSettingsURLString)!)
    }
  }
}
