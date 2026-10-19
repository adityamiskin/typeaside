import SwiftUI
import UIKit

struct KeyboardView: View {
  let state: KeyboardState
  let keyboard: KeyboardViewController

  var body: some View {
    VStack(spacing: 12) {
      Text(status)
        .font(.footnote)
        .foregroundStyle(.white.opacity(0.7))
        .lineLimit(1)
        .frame(height: 18)
      MicButton(phase: state.phase, action: keyboard.toggleDictation)
        .disabled(!state.hasFullAccess)
      HStack(spacing: 6) {
        if state.needsGlobe {
          GlobeKey(keyboard: keyboard)
            .background(Color.key, in: .rect(cornerRadius: 8))
            .frame(width: 48)
            .accessibilityLabel("Next keyboard")
        }
        Button("space") { keyboard.type(" ") }
          .buttonStyle(KeyStyle())
        DeleteKey(keyboard: keyboard)
          .frame(width: 56)
        Button { keyboard.type("\n") } label: { Image(systemName: "return") }
          .buttonStyle(KeyStyle())
          .frame(width: 72)
          .accessibilityLabel("Return")
      }
      .frame(height: 44)
    }
    .padding(.horizontal, 4)
    .padding(.vertical, 8)
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(.black)
    .foregroundStyle(.white)
  }

  private var status: String {
    guard state.hasFullAccess else { return "Allow Full Access in Settings to dictate" }
    switch state.phase {
    case .off: return state.session.message ?? "Tap the mic to start TypeAside"
    case .idle: return state.session.message ?? "Tap the mic to dictate"
    case .recording: return "Listening"
    case .processing: return "Transcribing"
    }
  }
}

extension Color {
  static let key = Color(white: 0.18)
  static let keyPressed = Color(white: 0.32)
}

private struct KeyStyle: ButtonStyle {
  func makeBody(configuration: Configuration) -> some View {
    configuration.label
      .font(.body)
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .background(configuration.isPressed ? Color.keyPressed : Color.key, in: .rect(cornerRadius: 8))
  }
}

private struct MicButton: View {
  let phase: DictationChannel.Phase
  let action: @MainActor () -> Void

  var body: some View {
    Button(action: action) {
      ZStack {
        Circle().fill(fill)
        Image(systemName: symbol)
          .font(.title2.weight(.semibold))
          .foregroundStyle(phase == .off || phase == .idle ? Color.black : Color.white)
      }
      .frame(width: 72, height: 72)
    }
    .buttonStyle(.plain)
    .disabled(phase == .processing)
    .accessibilityLabel(phase == .recording ? "Stop dictation" : "Dictate")
  }

  private var fill: Color {
    switch phase {
    case .recording: .red
    case .processing: .key
    case .off, .idle: .white
    }
  }

  private var symbol: String {
    switch phase {
    case .recording: "stop.fill"
    case .processing: "ellipsis"
    case .off, .idle: "mic.fill"
    }
  }
}

private struct DeleteKey: View {
  let keyboard: KeyboardViewController
  @State private var repeating: Task<Void, Never>?

  var body: some View {
    Image(systemName: "delete.left")
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .background(repeating == nil ? Color.key : Color.keyPressed, in: .rect(cornerRadius: 8))
      .contentShape(.rect)
      .onLongPressGesture(minimumDuration: .infinity, perform: {}) { pressing in
        repeating?.cancel()
        repeating = pressing ? deleteWhileHeld() : nil
      }
      .onDisappear {
        repeating?.cancel()
        repeating = nil
      }
      .accessibilityLabel("Delete")
      .accessibilityAddTraits(.isButton)
      .accessibilityAction { keyboard.deleteBackward() }
  }

  private func deleteWhileHeld() -> Task<Void, Never> {
    Task {
      UIImpactFeedbackGenerator(style: .light).impactOccurred()
      keyboard.deleteBackward()
      try? await Task.sleep(for: .milliseconds(450))
      while !Task.isCancelled {
        keyboard.deleteBackward()
        try? await Task.sleep(for: .milliseconds(80))
      }
    }
  }
}

// handleInputModeList(from:with:) needs the UIKit view and touch event, which SwiftUI does not expose.
private struct GlobeKey: UIViewRepresentable {
  let keyboard: UIInputViewController

  func makeUIView(context: Context) -> UIButton {
    let button = UIButton(type: .system)
    button.setImage(UIImage(systemName: "globe"), for: .normal)
    button.tintColor = .white
    button.addTarget(
      keyboard, action: #selector(UIInputViewController.handleInputModeList(from:with:)),
      for: .allTouchEvents)
    return button
  }

  func updateUIView(_ button: UIButton, context: Context) {}
}
