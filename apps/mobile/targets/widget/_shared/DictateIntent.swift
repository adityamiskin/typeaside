import AppIntents
import Foundation

@available(iOS 18.0, *)
struct DictateIntent: AppIntent {
  static let title: LocalizedStringResource = "Voice note"
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult {
    // Control Center's OpenURLIntent requires a universal link. The app bridge opens our scheme.
    UserDefaults(suiteName: "group.app.typeaside")?.set(
      true, forKey: "typeaside.widgets.dictateRequested"
    )
    NotificationCenter.default.post(
      name: Notification.Name("typeaside.widgets.dictateRequested"), object: nil
    )
    return .result()
  }
}
