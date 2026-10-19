import Foundation

// Compiled into both the app (Keyboard module) and the keyboard extension, which symlinks this file.
// Darwin notifications carry no payload, so state travels through the App Group container.
enum DictationChannel {
  static let sessionURL = URL(string: "app.typeaside://keyboard-session")!
  private static let appGroup = "group.app.typeaside"
  private static let container = FileManager.default.containerURL(
    forSecurityApplicationGroupIdentifier: appGroup)

  enum Phase: String, Codable {
    case off, idle, recording, processing
  }

  struct Transcript: Codable, Equatable {
    var id = UUID()
    let text: String
  }

  struct Session: Codable, Equatable {
    var phase = Phase.off
    var message: String?
    var transcript: Transcript?
  }

  struct Context: Codable {
    let beforeCursor: String
    let afterCursor: String
  }

  // Files rather than defaults: a reader woken by a notification must see the write that preceded it.
  static var session: Session {
    get { read("session.json") ?? Session() }
    set { write(newValue, to: "session.json") }
  }

  static var sessionRequestedAt: TimeInterval? {
    get { read("request.json") }
    set { write(newValue, to: "request.json") }
  }

  static var context: Context? {
    get { read("context.json") }
    set { write(newValue, to: "context.json") }
  }

  private static func read<Value: Decodable>(_ name: String) -> Value? {
    guard let url = container?.appending(path: name), let data = try? Data(contentsOf: url) else {
      return nil
    }
    return try? JSONDecoder().decode(Value.self, from: data)
  }

  private static func write(_ value: some Encodable, to name: String) {
    guard let url = container?.appending(path: name), let data = try? JSONEncoder().encode(value)
    else { return }
    try? data.write(to: url, options: .atomic)
  }

  enum Signal: String {
    case ping, start, stop, update

    fileprivate var name: CFNotificationName {
      CFNotificationName("app.typeaside.keyboard.\(rawValue)" as CFString)
    }
  }

  nonisolated(unsafe) private static var handlers: [String: @MainActor () -> Void] = [:]

  static func post(_ signal: Signal) {
    CFNotificationCenterPostNotification(
      CFNotificationCenterGetDarwinNotifyCenter(), signal.name, nil, nil, true)
  }

  @MainActor
  static func observe(_ signal: Signal, _ handler: @escaping @MainActor () -> Void) {
    // Later observers replace the handler instead of stacking a second delivery.
    guard handlers.updateValue(handler, forKey: signal.name.rawValue as String) == nil else { return }
    CFNotificationCenterAddObserver(
      CFNotificationCenterGetDarwinNotifyCenter(), nil,
      { _, _, name, _, _ in
        guard let name else { return }
        let key = name.rawValue as String
        Task { @MainActor in DictationChannel.handlers[key]?() }
      },
      signal.name.rawValue, nil, .deliverImmediately)
  }
}
