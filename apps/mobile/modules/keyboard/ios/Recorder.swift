import AVFoundation

// The engine runs for the whole session so the app keeps its background audio slot;
// buffers are only written while a file is open.
final class Recorder: @unchecked Sendable {
  let engine = AVAudioEngine()
  private let lock = NSLock()
  private var file: AVAudioFile?
  private var tapped = false

  func start() throws {
    let input = engine.inputNode
    let format = input.outputFormat(forBus: 0)
    guard format.sampleRate > 0 else { throw DictationError("Microphone unavailable") }
    input.installTap(onBus: 0, bufferSize: 2048, format: format) { [weak self] buffer, _ in
      self?.write(buffer)
    }
    tapped = true
    engine.prepare()
    try engine.start()
  }

  func stop() {
    if let url = endFile() { try? FileManager.default.removeItem(at: url) }
    if tapped { engine.inputNode.removeTap(onBus: 0) }
    tapped = false
    engine.stop()
  }

  func beginFile() throws {
    let format = engine.inputNode.outputFormat(forBus: 0)
    let url = FileManager.default.temporaryDirectory.appending(path: "\(UUID().uuidString).m4a")
    let file = try AVAudioFile(
      forWriting: url,
      settings: [
        AVFormatIDKey: kAudioFormatMPEG4AAC,
        AVSampleRateKey: format.sampleRate,
        AVNumberOfChannelsKey: format.channelCount,
      ],
      commonFormat: format.commonFormat,
      interleaved: format.isInterleaved)
    lock.withLock { self.file = file }
  }

  /// Releasing the AVAudioFile finalizes the m4a container.
  func endFile() -> URL? {
    lock.withLock {
      defer { file = nil }
      return file?.url
    }
  }

  private func write(_ buffer: AVAudioPCMBuffer) {
    lock.withLock {
      guard let file else { return }
      try? file.write(from: buffer)
    }
  }
}
