import AppKit

final class RichTextController: ObservableObject {
    fileprivate weak var textView: NSTextView?

    func toggleBold() {
        textView?.toggleBoldface(nil)
    }

    func toggleItalic() {
        textView?.toggleItalics(nil)
    }

    func toggleUnderline() {
        textView?.toggleUnderline(nil)
    }

    func insertBulletList() {
        guard let textView else { return }
        textView.insertText("• ", replacementRange: textView.selectedRange())
    }

    func insertHeading(level: Int) {
        guard let textView else { return }
        let hashes = String(repeating: "#", count: max(1, min(level, 6)))
        textView.insertText("\(hashes) ", replacementRange: textView.selectedRange())
    }
}
