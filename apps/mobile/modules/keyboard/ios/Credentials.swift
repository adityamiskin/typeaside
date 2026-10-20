import Foundation

struct Snippet: Codable {
  let trigger: String
  let text: String
}

struct Credentials: Codable {
  let token: String
  let apiUrl: String
  let dictionary: [String]
  let snippets: [Snippet]?

  private static var url: URL? {
    FileManager.default.containerURL(
      forSecurityApplicationGroupIdentifier: "group.app.typeaside"
    )?.appending(path: "credentials.json")
  }

  static func load() -> Credentials? {
    guard let url, let data = try? Data(contentsOf: url) else { return nil }
    return try? JSONDecoder().decode(Self.self, from: data)
  }

  func save() throws {
    guard let url = Self.url else { throw DictationError("App Group unavailable") }
    // Keep background uploads available after the device locks.
    try JSONEncoder().encode(self).write(
      to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
  }

  static func clear() {
    guard let url else { return }
    try? FileManager.default.removeItem(at: url)
  }
}
