import SharedTypes
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

struct ContentView: View {
    @ObservedObject var core: Core

    init(core: Core) {
        self.core = core
        // Insert initial document for testing
        core.update(.insert("Hello!"))
    }

    func editText(change: TextEditor.TextChange) {
        let location = UInt64(change.range.location), length = UInt64(change.range.length), text = change.replacementText

        if let text = text {
            core.update(.replace(location, location + length, text))
        } else {
            if length > 0 {
                core.update(.select(location, location + length))
            } else {
                core.update(.moveCursor(location))
            }
        }
    }

    var body: some View {
        TextEditor(text: core.view.text, selection: core.view.cursor.range())
            .onEdit { textChange in
                editText(change: textChange)
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
        ContentView(core:  Core())
    }
}
