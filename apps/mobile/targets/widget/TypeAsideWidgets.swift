import SwiftUI
import WidgetKit

private let dictateURL = URL(string: "app.typeaside://dictate?start=1")!
private let notesURL = URL(string: "app.typeaside://notes")!
private let newNoteURL = URL(string: "app.typeaside://notes/new")!

struct WidgetNote: Decodable, Identifiable {
  let id: String
  let title: String
  let preview: String
  let color: String
  let updatedAt: Double

  var displayTitle: String {
    title.isEmpty ? String(preview.split(separator: "\n", omittingEmptySubsequences: false).first ?? "") : title
  }

  var displayPreview: String {
    title.isEmpty ? preview.split(separator: "\n", omittingEmptySubsequences: false).dropFirst().joined(separator: " ") : preview.replacingOccurrences(of: "\n", with: " ")
  }

  var tint: Color {
    switch color {
    case "sky": .blue
    case "lilac": .purple
    case "sage": .green
    case "sand": .orange
    case "rose": .pink
    default: .secondary
    }
  }

  var url: URL {
    notesURL.appendingPathComponent(id)
  }
}

struct Snapshot: Decodable {
  let wordsLeft: Int?
  let notes: [WidgetNote]

  static func read() -> Snapshot? {
    guard let json = UserDefaults(suiteName: "group.app.typeaside")?
      .string(forKey: "typeaside.widgets.snapshot") else { return nil }
    return try? JSONDecoder().decode(Self.self, from: Data(json.utf8))
  }
}

struct Entry: TimelineEntry {
  let date: Date
  let snapshot: Snapshot?
}

struct Provider: TimelineProvider {
  func placeholder(in context: Context) -> Entry {
    Entry(date: .now, snapshot: Snapshot(wordsLeft: nil, notes: []))
  }

  func getSnapshot(in context: Context, completion: @escaping (Entry) -> Void) {
    completion(Entry(date: .now, snapshot: Snapshot.read()))
  }

  func getTimeline(in context: Context, completion: @escaping (Timeline<Entry>) -> Void) {
    completion(Timeline(entries: [Entry(date: .now, snapshot: Snapshot.read())], policy: .never))
  }
}

struct DictateView: View {
  @Environment(\.widgetFamily) private var family
  let entry: Entry

  var body: some View {
    Group {
      if family == .accessoryCircular {
        ZStack {
          AccessoryWidgetBackground()
          Image(systemName: "mic.fill").font(.title2).widgetAccentable()
        }
      } else {
        VStack(alignment: .leading) {
          Image(systemName: "mic.fill")
            .font(.title2)
            .foregroundStyle(.white)
            .widgetAccentable()
            .frame(width: 44, height: 44)
            .background(.tint, in: Circle())
          Spacer()
          Text("Voice note").font(.headline)
          if let words = entry.snapshot?.wordsLeft {
            Text("\(words.formatted()) words left").font(.caption).foregroundStyle(.secondary)
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
      }
    }
    .widgetURL(dictateURL)
    .containerBackground(.fill.tertiary, for: .widget)
  }
}

struct RecentNotesView: View {
  @Environment(\.widgetFamily) private var family
  let entry: Entry

  var body: some View {
    Group {
      if let snapshot = entry.snapshot {
        VStack(alignment: .leading, spacing: 0) {
          HStack {
            Text("Notes").font(.headline)
            Spacer()
            Link(destination: newNoteURL) {
              Image(systemName: "plus.circle.fill").font(.title2).foregroundStyle(Color.accentColor)
            }.accessibilityLabel("New note")
          }
          if snapshot.notes.isEmpty {
            Text("No notes yet").font(.subheadline).foregroundStyle(.secondary)
              .frame(maxWidth: .infinity, maxHeight: .infinity)
          } else {
            let notes = Array(snapshot.notes.prefix(family == .systemLarge ? 6 : 3))
            ForEach(Array(notes.enumerated()), id: \.element.id) { index, note in
              if index > 0 { Divider().padding(.vertical, 6) }
              Link(destination: note.url) {
                HStack(spacing: 8) {
                  Circle().fill(note.tint).frame(width: 8, height: 8)
                  VStack(alignment: .leading, spacing: 1) {
                    Text(note.displayTitle).font(.subheadline.weight(.semibold)).lineLimit(1)
                    if !note.displayPreview.isEmpty {
                      Text(note.displayPreview).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                  }
                  Spacer(minLength: 0)
                }
              }.buttonStyle(.plain)
            }
            .padding(.top, 2)
            Spacer(minLength: 0)
          }
        }
      } else {
        Text("Open TypeAside to sign in").font(.subheadline).foregroundStyle(.secondary)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
    }
    .widgetURL(notesURL)
    .containerBackground(.fill.tertiary, for: .widget)
  }
}

struct DictateWidget: Widget {
  var body: some WidgetConfiguration {
    StaticConfiguration(kind: "app.typeaside.dictate", provider: Provider()) { DictateView(entry: $0) }
      .configurationDisplayName("Voice note")
      .description("Start a voice note in TypeAside")
      .supportedFamilies([.systemSmall, .accessoryCircular])
  }
}

struct NotesWidget: Widget {
  var body: some WidgetConfiguration {
    StaticConfiguration(kind: "app.typeaside.notes", provider: Provider()) { RecentNotesView(entry: $0) }
      .configurationDisplayName("Notes")
      .description("Your recent TypeAside notes")
      .supportedFamilies([.systemMedium, .systemLarge])
  }
}

@available(iOS 18.0, *)
struct DictateControl: ControlWidget {
  var body: some ControlWidgetConfiguration {
    StaticControlConfiguration(kind: "app.typeaside.dictate.control") {
      ControlWidgetButton(action: DictateIntent()) {
        Label("Voice note", systemImage: "mic.fill")
      }
    }.displayName("Voice note")
  }
}

@main
struct TypeAsideWidgets: WidgetBundle {
  var body: some Widget {
    DictateWidget()
    NotesWidget()
    if #available(iOS 18.0, *) { DictateControl() }
  }
}
