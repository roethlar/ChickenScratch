import SwiftUI
import AppKit

struct RichTextEditor: NSViewRepresentable {
    @Binding var attributedText: NSAttributedString
    let controller: RichTextController
    var onTextChange: (NSAttributedString) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.drawsBackground = false
        scrollView.hasVerticalScroller = true

        let textView = NSTextView()
        textView.delegate = context.coordinator
        textView.isRichText = true
        textView.importsGraphics = false
        textView.allowsUndo = true
        textView.usesFindPanel = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = true
        textView.font = .systemFont(ofSize: 14)
        textView.textColor = .textColor
        textView.backgroundColor = .clear
        textView.string = attributedText.string
        textView.textStorage?.setAttributedString(attributedText)

        controller.textView = textView

        scrollView.documentView = textView
        return scrollView
    }

    func updateNSView(_ nsView: NSScrollView, context: Context) {
        guard let textView = nsView.documentView as? NSTextView else { return }
        if !context.coordinator.isEditing {
            let current = textView.attributedString()
            if !current.isEqual(to: attributedText) {
                textView.textStorage?.setAttributedString(attributedText)
            }
        }
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: RichTextEditor
        var isEditing = false

        init(parent: RichTextEditor) {
            self.parent = parent
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            isEditing = true
            let attributed = textView.attributedString()
            parent.attributedText = attributed
            parent.onTextChange(attributed)
            isEditing = false
        }
    }
}
