import Foundation
import AppKit

struct MarkdownTransformer {
    func attributedString(fromMarkdown markdown: String) -> NSAttributedString {
        do {
            let attributed = try AttributedString(
                markdown: markdown,
                options: AttributedString.MarkdownParsingOptions(
                    allowsExtendedAttributes: true,
                    interpretedSyntax: .full
                )
            )
            return NSAttributedString(attributed)
        } catch {
            return NSAttributedString(string: markdown)
        }
    }

    func markdown(from attributedString: NSAttributedString) -> String {
        let fullRange = NSRange(location: 0, length: attributedString.length)
        guard fullRange.length > 0 else { return "" }

        let fontManager = NSFontManager.shared
        var result = ""
        let baseString = attributedString.string as NSString

        attributedString.enumerateAttributes(in: fullRange, options: []) { attributes, range, _ in
            guard range.length > 0 else { return }
            let substring = baseString.substring(with: range)
            let escaped = escapeMarkdown(substring)

            let font = attributes[.font] as? NSFont
            let traits: NSFontTraitMask
            if let font {
                traits = fontManager.traits(of: font)
            } else {
                traits = []
            }
            let isBold = traits.contains(.boldFontMask)
            let isItalic = traits.contains(.italicFontMask)
            let isUnderlined = (attributes[.underlineStyle] as? Int ?? 0) != 0

            var prefix = ""
            var suffix = ""

            if isBold && isItalic {
                prefix += "***"
                suffix = "***" + suffix
            } else if isBold {
                prefix += "**"
                suffix = "**" + suffix
            } else if isItalic {
                prefix += "*"
                suffix = "*" + suffix
            }

            if isUnderlined {
                prefix += "<u>"
                suffix = "</u>" + suffix
            }

            result += prefix + escaped + suffix
        }

        return result
    }

    private func escapeMarkdown(_ text: String) -> String {
        var escaped = text
        let replacements: [(String, String)] = [
            ("\\", "\\\\"),
            ("`", "\\`"),
            ("*", "\\*"),
            ("_", "\\_")
        ]
        for (target, replacement) in replacements {
            escaped = escaped.replacingOccurrences(of: target, with: replacement)
        }
        return escaped
    }
}
