import AVFoundation
import UIKit

// The keyboard extension cannot record, so the app holds an idle microphone while a session is live
// and records on the keyboard's signal, even when backgrounded (UIBackgroundModes audio).
@MainActor
final class KeyboardSession {
  static let shared = KeyboardSession()

  private let recorder = Recorder()
  private var timeout: Task<Void, Never>?
  private var upload: Task<Void, Never>?
  private var observers: [NSObjectProtocol] = []
  private var state = DictationChannel.Session() {
    didSet {
      DictationChannel.session = state
      DictationChannel.post(.update)
    }
  }

  func activate() {
    guard observers.isEmpty else { return }
    state = .init()
    DictationChannel.observe(.ping) { DictationChannel.post(.update) }
    DictationChannel.observe(.start) { [self] in startRecording() }
    DictationChannel.observe(.stop) { [self] in stopRecording() }
    let center = NotificationCenter.default
    observers = [
      center.addObserver(
        forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main
      ) { _ in MainActor.assumeIsolated { KeyboardSession.shared.beginIfRequested() } },
      center.addObserver(
        forName: AVAudioSession.interruptionNotification, object: nil, queue: .main
      ) { _ in MainActor.assumeIsolated { KeyboardSession.shared.end() } },
      center.addObserver(
        forName: .AVAudioEngineConfigurationChange, object: recorder.engine, queue: .main
      ) { _ in MainActor.assumeIsolated { KeyboardSession.shared.end() } },
    ]
    if UIApplication.shared.applicationState == .active { beginIfRequested() }
  }

  func end() {
    timeout?.cancel()
    upload?.cancel()
    upload = nil
    guard state.phase != .off else { return }
    recorder.stop()
    try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    state = .init()
  }

  // The keyboard marks the request before opening the app; the URL itself only brings the app forward.
  private func beginIfRequested() {
    guard let requestedAt = DictationChannel.sessionRequestedAt,
      Date().timeIntervalSince1970 - requestedAt < 60
    else { return }
    DictationChannel.sessionRequestedAt = nil
    AVAudioSession.sharedInstance().requestRecordPermission { granted in
      Task { @MainActor in KeyboardSession.shared.begin(granted: granted) }
    }
  }

  private func begin(granted: Bool) {
    guard state.phase == .off else { return }
    guard granted else {
      state = .init(message: "Allow microphone access for TypeAside in Settings")
      return
    }
    do {
      let session = AVAudioSession.sharedInstance()
      try session.setCategory(.playAndRecord, mode: .default, options: [.mixWithOthers, .defaultToSpeaker])
      try session.setActive(true)
      try recorder.start()
      state = .init(phase: .idle)
      scheduleTimeout()
    } catch {
      recorder.stop()
      try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
      state = .init(message: "Microphone unavailable")
    }
  }

  private func startRecording() {
    guard state.phase == .idle else { return }
    guard Credentials.load() != nil else {
      state = .init(phase: .idle, message: "Sign in to TypeAside")
      return
    }
    do {
      try recorder.beginFile()
      timeout?.cancel()
      state = .init(phase: .recording)
      timeout = Task {
        try? await Task.sleep(for: .seconds(4 * 60))
        if !Task.isCancelled { stopRecording() }
      }
    } catch {
      state = .init(phase: .idle, message: "Could not start recording")
    }
  }

  private func stopRecording() {
    guard state.phase == .recording, let audio = recorder.endFile() else { return }
    timeout?.cancel()
    state = .init(phase: .processing)
    let context = DictationChannel.context
    upload = Task {
      defer { try? FileManager.default.removeItem(at: audio) }
      let next: DictationChannel.Session
      do {
        guard let credentials = Credentials.load() else { throw DictationError("Sign in to TypeAside") }
        let text = try await Dictation.send(audio, credentials: credentials, context: context)
        next = text.isEmpty
          ? .init(phase: .idle, message: "No speech heard")
          : .init(phase: .idle, transcript: .init(text: text))
      } catch {
        next = .init(phase: .idle, message: (error as? DictationError)?.message ?? "Dictation failed")
      }
      guard !Task.isCancelled, state.phase == .processing else { return }
      state = next
      scheduleTimeout()
    }
  }

  private func scheduleTimeout() {
    timeout?.cancel()
    timeout = Task {
      try? await Task.sleep(for: .seconds(5 * 60))
      if !Task.isCancelled { end() }
    }
  }
}
