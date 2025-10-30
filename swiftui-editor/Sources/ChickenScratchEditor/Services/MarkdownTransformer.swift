import Foundation
import AppKit

struct MarkdownTransformer {
    func attributedString(fromMarkdown markdown: String) -> NSAttributedString {
        do {
            let attributed = try AttributedString(
                markdown: markdown,
                options: AttributedString.MarkdownParsingOptions(
                    interpretedSyntax: .full,
                    allowsExtendedAttributes: true
                )
            )
            return NSAttributedString(attributed)
        } catch {
            return NSAttributedString(string: markdown)
        }
    }

    func markdown(from attributedString: NSAttributedString) -> String {
        do {
            let attributed = try AttributedString(attributedString, including: \.appKit)
            if let markdown = attributed.markdownRepresentation {
                return markdown
            }
        } catch {
            // Fall through to plain text representation
        }
        return attributedString.string
    }
}
