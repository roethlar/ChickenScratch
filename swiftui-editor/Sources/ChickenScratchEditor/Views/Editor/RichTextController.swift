import AppKit

final class RichTextController: ObservableObject {
    public weak var textView: NSTextView?

    func toggleBold() {
        toggleFontTrait(NSFontTraitMask.boldFontMask)
    }

    func toggleItalic() {
        toggleFontTrait(NSFontTraitMask.italicFontMask)
    }

    func toggleUnderline() {
        guard let textView else { return }
        let range = textView.selectedRange()
        let storage = textView.textStorage
        let currentStyle = currentUnderlineStyle(in: textView, range: range)
        let shouldEnable = currentStyle == 0

        if range.length > 0, let storage {
            storage.beginEditing()
            if shouldEnable {
                storage.addAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, range: range)
            } else {
                storage.removeAttribute(.underlineStyle, range: range)
            }
            storage.endEditing()
        } else {
            if shouldEnable {
                textView.typingAttributes[.underlineStyle] = NSUnderlineStyle.single.rawValue
            } else {
                textView.typingAttributes.removeValue(forKey: .underlineStyle)
            }
        }
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

    private func toggleFontTrait(_ trait: NSFontTraitMask) {
        guard let textView else { return }
        let manager = NSFontManager.shared
        let range = textView.selectedRange()

        if range.length > 0, let storage = textView.textStorage {
            storage.beginEditing()
            storage.enumerateAttribute(.font, in: range, options: []) { value, subrange, _ in
                let font = (value as? NSFont) ?? textView.font ?? NSFont.systemFont(ofSize: NSFont.systemFontSize)
                manager.setSelectedFont(font, isMultiple: false)
                let traits = manager.traits(of: font)
                let toggled: NSFont
                if traits.contains(trait) {
                    toggled = manager.convert(font, toNotHaveTrait: trait)
                } else {
                    toggled = manager.convert(font, toHaveTrait: trait)
                }
                storage.addAttribute(.font, value: toggled, range: subrange)
            }
            storage.endEditing()
        } else {
            let baseFont = (textView.typingAttributes[.font] as? NSFont) ?? textView.font ?? NSFont.systemFont(ofSize: NSFont.systemFontSize)
            manager.setSelectedFont(baseFont, isMultiple: false)
            let traits = manager.traits(of: baseFont)
            let toggled: NSFont
            if traits.contains(trait) {
                toggled = manager.convert(baseFont, toNotHaveTrait: trait)
            } else {
                toggled = manager.convert(baseFont, toHaveTrait: trait)
            }
            textView.typingAttributes[.font] = toggled
            textView.font = toggled
        }
    }

    private func currentUnderlineStyle(in textView: NSTextView, range: NSRange) -> Int {
        if range.length > 0, let storage = textView.textStorage, storage.length > 0 {
            let index = min(range.location, max(storage.length - 1, 0))
            return storage.attribute(.underlineStyle, at: index, effectiveRange: nil) as? Int ?? 0
        }
        return textView.typingAttributes[.underlineStyle] as? Int ?? 0
    }
}
