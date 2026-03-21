import SwiftUI
import App

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
        core.update(.open)
    }

    func editText(change: NoteTextEditor.TextChange) {
        let location = UInt64(change.range.location)
        let length = UInt64(change.range.length)
        let text = change.replacementText

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
        NoteTextEditor(text: core.view.text, selection: core.view.cursor.range())
            .onEdit { textChange in
                editText(change: textChange)
            }
            .padding()
    }
}

// MARK: - Shared types

extension NoteTextEditor {
    struct TextChange {
        var range: NSRange
        var replacementText: String?
    }

    func onEdit(task: @escaping (TextChange) -> Void) -> NoteTextEditor {
        var modified = self
        modified.onEditHandler = task
        return modified
    }
}

// MARK: - Platform-specific NoteTextEditor

#if canImport(UIKit)

struct NoteTextEditor: UIViewRepresentable {
    var text: String
    var selection: NSRange
    var onEditHandler: ((TextChange) -> Void)?

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeUIView(context: Context) -> UITextView {
        context.coordinator.textView
    }

    func updateUIView(_ textView: UITextView, context: Context) {
        textView.text = text
        textView.selectedRange = selection
        context.coordinator.onEdit = onEditHandler
    }

    class Coordinator: NSObject, UITextViewDelegate {
        lazy var textView: UITextView = {
            let textView = UITextView()
            textView.font = UIFont.preferredFont(forTextStyle: .body)
            textView.delegate = self
            return textView
        }()

        var onEdit: ((TextChange) -> Void)?
        private var changes: [TextChange] = []

        func textView(
            _: UITextView,
            shouldChangeTextIn range: NSRange,
            replacementText text: String
        ) -> Bool {
            changes.append(TextChange(range: range, replacementText: text))
            return true
        }

        func textViewDidChange(_: UITextView) {
            for change in changes {
                onEdit?(change)
            }
            changes = []
        }

        func textViewDidChangeSelection(_ textView: UITextView) {
            onEdit?(TextChange(range: textView.selectedRange, replacementText: .none))
        }
    }
}

#else

struct NoteTextEditor: NSViewRepresentable {
    var text: String
    var selection: NSRange
    var onEditHandler: ((TextChange) -> Void)?

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        let textView = context.coordinator.textView
        scrollView.documentView = textView
        scrollView.hasVerticalScroller = true
        textView.autoresizingMask = [.width, .height]
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else { return }
        if textView.string != text {
            textView.string = text
        }
        textView.setSelectedRange(selection)
        context.coordinator.onEdit = onEditHandler
    }

    class Coordinator: NSObject, NSTextViewDelegate {
        lazy var textView: NSTextView = {
            let textView = NSTextView()
            textView.font = NSFont.systemFont(ofSize: NSFont.systemFontSize)
            textView.isEditable = true
            textView.isSelectable = true
            textView.isRichText = false
            textView.delegate = self
            return textView
        }()

        var onEdit: ((TextChange) -> Void)?
        private var changes: [TextChange] = []

        func textView(
            _ textView: NSTextView,
            shouldChangeTextIn range: NSRange,
            replacementString text: String?
        ) -> Bool {
            if let text = text {
                changes.append(TextChange(range: range, replacementText: text))
            }
            return true
        }

        func textDidChange(_ notification: Notification) {
            for change in changes {
                onEdit?(change)
            }
            changes = []
        }

        func textViewDidChangeSelection(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            onEdit?(TextChange(range: textView.selectedRange(), replacementText: .none))
        }
    }
}

#endif

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(core: Core())
    }
}
