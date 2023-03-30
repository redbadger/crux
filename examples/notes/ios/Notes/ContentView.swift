//
//  ContentView.swift
//  Notes
//
//  Created by Stuart Harris on 30/03/2023.
//

import SwiftUI

extension TextCursor {
    func range() -> NSRange {
        switch self {
        case let .position(idx):
            return NSRange(location: Int(idx), length: 0)
        case let .selection(range):
            return NSRange(location: Int(range.start), length: Int(range.end) - Int(range.start))
        }
    }
}

typealias Uuid = [UInt8]

enum CoreEvent {
    case event(Event)
    // TODO: response
}

class Core: ObservableObject {
    @Published var view: ViewModel

    var num = 1

    init() {
        view = ViewModel(text: "", cursor: .position(0)) // this will get replaced by the next call

        // Insert initial document for testing
        update(event: .event(.insert("Hello from the core!")))
    }

    func editText(change: TextEditor.TextChange) {
        let location = UInt64(change.range.location), length = UInt64(change.range.length), text = change.replacementText

        if let text = text {
            update(event: .event(.replace(location, location + length, text)))
        } else {
            if length > 0 {
                update(event: .event(.select(location, location + length)))
            } else {
                update(event: .event(.moveCursor(location)))
            }
        }
    }

    func update(event: CoreEvent) {
        var requests: [Request]

        switch event {
        case let .event(evt):
            let bytes = Notes.processEvent(try! evt.bcsSerialize())
            requests = try! [Request].bcsDeserialize(input: bytes)
        }

        for req in requests {
            switch req.effect {
            case .render:
                view = try! ViewModel.bcsDeserialize(input: Notes.view())
            case let .pubSub(.publish(bytes)):
                print(["Publish", bytes.count, "bytes"])
            case .pubSub(.subscribe):
                print("Subscribe")
            case .keyValue(_): ()
            case .timer(_): ()
            }
        }
    }
}

struct ContentView: View {
    @StateObject private var core = Core()

    var body: some View {
        TextEditor(text: core.view.text, selection: core.view.cursor.range())
            .onEdit { textChange in
                core.editText(change: textChange)
            }
            .padding()
    }
}

struct TextEditor: UIViewRepresentable {
    var text: String
    var selection: NSRange

    var _onEdit: ((TextChange) -> Void)?

    struct TextChange {
        var range: NSRange
        var replacementText: String?
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeUIView(context: Context) -> UITextView {
        context.coordinator.textView
    }

    func updateUIView(_ tv: UITextView, context: Context) {
        tv.text = text
        tv.selectedRange = selection
        context.coordinator.onEdit = _onEdit
    }

    class Coordinator: NSObject, UITextViewDelegate {
        lazy var textView: UITextView = {
            let tv = UITextView()

            tv.font = UIFont.preferredFont(forTextStyle: .body)
            tv.delegate = self

            return tv
        }()

        var onEdit: ((TextChange) -> Void)?
        private var changes: [TextChange] = []

        func textView(_: UITextView, shouldChangeTextIn range: NSRange, replacementText text: String) -> Bool {
            let change = TextChange(range: range, replacementText: text)

            // Stage the change
            changes.append(change)

            return true
        }

        func textViewDidChange(_: UITextView) {
            // Commit changes
            for change in changes {
                onEdit?(change)
            }

            changes = []
        }

        func textViewDidChangeSelection(_ textView: UITextView) {
            let change = TextChange(range: textView.selectedRange, replacementText: .none)

            onEdit?(change)
        }
    }
}

extension TextEditor {
    func onEdit(task: @escaping (TextChange) -> Void) -> TextEditor {
        var modified = self
        modified._onEdit = task

        return modified
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
