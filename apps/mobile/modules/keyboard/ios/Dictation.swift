import Foundation

struct DictationError: Error {
  let message: String
  init(_ message: String) { self.message = message }
}

// Mirrors contextInput in apps/api/src/dictate.ts.
private struct DictationContext: Encodable {
  let mode = "dictate"
  let category = "personal"
  let tone = "casual"
  let translateTo = "English"
  let names: [String] = []
  let dictionary: [String]
  let beforeCursor: String?
  let afterCursor: String?
}

private struct Success: Decodable {
  let text: String
}

private struct Failure: Decodable {
  struct Detail: Decodable { let message: String }
  let error: Detail
}

enum Dictation {
  static func send(
    _ audio: URL, credentials: Credentials, context: DictationChannel.Context?
  ) async throws -> String {
    let encoder = JSONEncoder()
    encoder.keyEncodingStrategy = .convertToSnakeCase
    let json = try encoder.encode(
      DictationContext(
        dictionary: credentials.dictionary,
        beforeCursor: context?.beforeCursor,
        afterCursor: context?.afterCursor))

    let boundary = UUID().uuidString
    var body = Data()
    body.append(Data("--\(boundary)\r\nContent-Disposition: form-data; name=\"context\"\r\n\r\n".utf8))
    body.append(json)
    body.append(
      Data(
        "\r\n--\(boundary)\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"audio.m4a\"\r\nContent-Type: audio/mp4\r\n\r\n"
          .utf8))
    body.append(try Data(contentsOf: audio))
    body.append(Data("\r\n--\(boundary)--\r\n".utf8))

    guard let url = URL(string: "\(credentials.apiUrl)/v1/dictate") else {
      throw DictationError("Invalid API URL")
    }
    var request = URLRequest(url: url, timeoutInterval: 60)
    request.httpMethod = "POST"
    request.setValue("Bearer \(credentials.token)", forHTTPHeaderField: "Authorization")
    request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")

    let result: (Data, URLResponse)
    do {
      result = try await URLSession.shared.upload(for: request, from: body)
    } catch {
      throw DictationError("No connection")
    }
    let (data, response) = result
    let status = (response as? HTTPURLResponse)?.statusCode ?? 0
    if status == 200, let success = try? JSONDecoder().decode(Success.self, from: data) {
      return success.text
    }
    if status == 401 { throw DictationError("Signed out. Open TypeAside to reconnect") }
    if status == 402 {
      let failure = try? JSONDecoder().decode(Failure.self, from: data)
      throw DictationError(failure?.error.message ?? "Dictation limit reached. Open TypeAside")
    }
    if let failure = try? JSONDecoder().decode(Failure.self, from: data) {
      throw DictationError(failure.error.message)
    }
    throw DictationError("Dictation failed (\(status))")
  }
}
