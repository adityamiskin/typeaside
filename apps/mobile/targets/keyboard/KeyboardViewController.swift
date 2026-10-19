import SwiftUI
import UIKit

@MainActor @Observable
final class KeyboardState {
  var session = DictationChannel.Session()
  // The session file outlives the app process, so it is trusted only after the app answers a ping.
  var live = false
  var hasFullAccess = true
  var needsGlobe = true

  var phase: DictationChannel.Phase { live ? session.phase : .off }
}

final class KeyboardViewController: UIInputViewController {
  private let state = KeyboardState()
  private let haptics = UIImpactFeedbackGenerator(style: .light)
  private var insertedTranscript: UUID?
  private var watchdog: Task<Void, Never>?
  private var visible = false
  private var recordingDocument: UUID?

  override func viewDidLoad() {
    super.viewDidLoad()
    let host = UIHostingController(rootView: KeyboardView(state: state, keyboard: self))
    host.view.backgroundColor = .black
    host.view.translatesAutoresizingMaskIntoConstraints = false
    addChild(host)
    view.addSubview(host.view)
    NSLayoutConstraint.activate([
      host.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
      host.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
      host.view.topAnchor.constraint(equalTo: view.topAnchor),
      host.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
      host.view.heightAnchor.constraint(equalToConstant: 216),
    ])
    host.didMove(toParent: self)
    DictationChannel.observe(.update) { [weak self] in self?.sessionChanged() }
  }

  override func viewWillAppear(_ animated: Bool) {
    super.viewWillAppear(animated)
    visible = true
    state.hasFullAccess = hasFullAccess
    state.live = false
    state.session = DictationChannel.session
    // A transcript that finished while the keyboard was hidden belongs to another text field.
    insertedTranscript = state.session.transcript?.id
    DictationChannel.post(.ping)
  }

  override func viewWillDisappear(_ animated: Bool) {
    super.viewWillDisappear(animated)
    visible = false
    watchdog?.cancel()
    if state.phase == .recording { DictationChannel.post(.stop) }
  }

  override func viewWillLayoutSubviews() {
    super.viewWillLayoutSubviews()
    state.needsGlobe = needsInputModeSwitchKey
  }

  func toggleDictation() {
    haptics.impactOccurred()
    switch state.phase {
    case .off: requestSession()
    case .idle:
      recordingDocument = textDocumentProxy.documentIdentifier
      let before = textDocumentProxy.documentContextBeforeInput ?? ""
      let after = textDocumentProxy.documentContextAfterInput ?? ""
      DictationChannel.context = .init(
        beforeCursor: String(before.unicodeScalars.suffix(500)),
        afterCursor: String(after.unicodeScalars.prefix(200)))
      send(.start)
    case .recording: send(.stop)
    case .processing: break
    }
  }

  func type(_ text: String) {
    haptics.impactOccurred()
    textDocumentProxy.insertText(text)
  }

  func deleteBackward() {
    textDocumentProxy.deleteBackward()
  }

  // If the app was killed, nothing answers; fall back to opening it on the next tap.
  private func send(_ signal: DictationChannel.Signal) {
    DictationChannel.post(signal)
    watchdog?.cancel()
    watchdog = Task {
      try? await Task.sleep(for: .seconds(1.5))
      if !Task.isCancelled { state.live = false }
    }
  }

  private func sessionChanged() {
    watchdog?.cancel()
    state.live = true
    state.session = DictationChannel.session
    if let transcript = state.session.transcript, transcript.id != insertedTranscript {
      insertedTranscript = transcript.id
      guard visible, textDocumentProxy.documentIdentifier == recordingDocument else { return }
      let before = textDocumentProxy.documentContextBeforeInput ?? ""
      let spaced = before.last.map { !$0.isWhitespace } ?? false
      textDocumentProxy.insertText(spaced ? " " + transcript.text : transcript.text)
      UINotificationFeedbackGenerator().notificationOccurred(.success)
    }
  }

  // Extensions cannot use UIApplication.shared; the host's application object is reachable
  // through the responder chain instead.
  private func requestSession() {
    DictationChannel.sessionRequestedAt = Date().timeIntervalSince1970
    var responder: UIResponder? = self
    while let current = responder {
      if let application = current as? UIApplication {
        application.open(DictationChannel.sessionURL, options: [:]) { [weak self] opened in
          if !opened {
            self?.state.session.message = "Open TypeAside to start a keyboard session"
          }
        }
        return
      }
      responder = current.next
    }
    state.session.message = "Open TypeAside to start a keyboard session"
  }
}
