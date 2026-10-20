import ExpoModulesCore
import UIKit
import WidgetKit

private let group = "group.app.typeaside"
private let snapshotKey = "typeaside.widgets.snapshot"
private let dictateRequestKey = "typeaside.widgets.dictateRequested"
private let dictateNotification = Notification.Name("typeaside.widgets.dictateRequested")

private struct Snapshot: Codable {
  let wordsLeft: Int?
  var notes: [Note]

  struct Note: Codable {
    let id: String
    let title: String
    let preview: String
    let color: String
    let updatedAt: Double
  }
}

public final class WidgetsModule: Module {
  private var observer: NSObjectProtocol?

  public func definition() -> ModuleDefinition {
    Name("Widgets")

    OnCreate { [weak self] in
      self?.observer = NotificationCenter.default.addObserver(
        forName: dictateNotification, object: nil, queue: .main
      ) { _ in
        Task { @MainActor in Self.openPendingDictation() }
      }
      Task { @MainActor in Self.openPendingDictation() }
    }

    OnAppBecomesActive {
      Task { @MainActor in Self.openPendingDictation() }
    }

    OnDestroy { [weak self] in
      if let observer = self?.observer { NotificationCenter.default.removeObserver(observer) }
    }

    AsyncFunction("setSnapshot") { (json: String) in
      var snapshot = try JSONDecoder().decode(Snapshot.self, from: Data(json.utf8))
      snapshot.notes = Array(snapshot.notes.sorted { $0.updatedAt > $1.updatedAt }.prefix(6))
      let data = try JSONEncoder().encode(snapshot)
      UserDefaults(suiteName: group)?.set(String(decoding: data, as: UTF8.self), forKey: snapshotKey)
      WidgetCenter.shared.reloadAllTimelines()
    }

    AsyncFunction("clear") {
      let defaults = UserDefaults(suiteName: group)
      defaults?.removeObject(forKey: snapshotKey)
      defaults?.removeObject(forKey: dictateRequestKey)
      WidgetCenter.shared.reloadAllTimelines()
    }
  }

  @MainActor
  private static func openPendingDictation() {
    let defaults = UserDefaults(suiteName: group)
    guard UIApplication.shared.applicationState == .active,
      defaults?.bool(forKey: dictateRequestKey) == true else { return }
    defaults?.removeObject(forKey: dictateRequestKey)
    UIApplication.shared.open(URL(string: "app.typeaside://dictate?start=1")!)
  }
}
