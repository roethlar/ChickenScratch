import ChiknKit
import SwiftUI

struct InspectorView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        if let doc = store.activeDocument() {
            InspectorForm(document: doc)
        } else {
            VStack {
                Spacer()
                Text("No document selected")
                    .foregroundStyle(.secondary)
                Spacer()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

private struct InspectorForm: View {
    let document: Document

    var body: some View {
        GlassEffectContainer(spacing: 12) {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    section(title: "Synopsis") {
                        Text(document.meta.synopsis ?? "No synopsis.")
                            .foregroundStyle(document.meta.synopsis == nil ? .secondary : .primary)
                            .textSelection(.enabled)
                    }

                    labelStatus

                    if !document.meta.keywords.isEmpty {
                        section(title: "Keywords") {
                            FlowRow(spacing: 6) {
                                ForEach(document.meta.keywords, id: \.self) { kw in
                                    Text(kw)
                                        .font(.caption)
                                        .padding(.horizontal, 10)
                                        .padding(.vertical, 4)
                                        .capsuleGlass()
                                }
                            }
                        }
                    }

                    section(title: "Compile") {
                        HStack {
                            Image(systemName: document.meta.includeInCompile ? "checkmark.circle.fill" : "circle")
                                .foregroundStyle(document.meta.includeInCompile ? .green : .secondary)
                            Text(document.meta.includeInCompile ? "Included" : "Excluded")
                        }
                        if let target = document.meta.wordCountTarget {
                            Text("Target: \(target) words")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }

                    section(title: "Word count") {
                        Text("\(wordCount(document.content))")
                            .font(.title2.weight(.medium))
                            .monospacedDigit()
                    }
                }
                .padding(20)
            }
            .panelGlass(cornerRadius: 20)
            .padding(12)
        }
    }

    @ViewBuilder
    private var labelStatus: some View {
        HStack(spacing: 12) {
            if let label = document.meta.label {
                chip(label, systemImage: "tag")
            }
            if let status = document.meta.status {
                chip(status, systemImage: "circle.lefthalf.filled")
            }
        }
    }

    private func chip(_ text: String, systemImage: String) -> some View {
        Label(text, systemImage: systemImage)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .capsuleGlass()
    }

    private func section<Content: View>(title: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title.uppercased())
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
            content()
        }
    }

    private func wordCount(_ text: String) -> Int {
        text.split { $0.isWhitespace || $0.isNewline }.count
    }
}

/// Minimal flow layout used for keyword chips.
private struct FlowRow: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxWidth = proposal.width ?? .infinity
        var width: CGFloat = 0
        var rowWidth: CGFloat = 0
        var height: CGFloat = 0
        var rowHeight: CGFloat = 0

        for sub in subviews {
            let size = sub.sizeThatFits(.unspecified)
            if rowWidth + size.width > maxWidth, rowWidth > 0 {
                width = max(width, rowWidth - spacing)
                height += rowHeight + spacing
                rowWidth = 0
                rowHeight = 0
            }
            rowWidth += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        width = max(width, rowWidth - spacing)
        height += rowHeight
        return CGSize(width: width, height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX
        var y = bounds.minY
        var rowHeight: CGFloat = 0

        for sub in subviews {
            let size = sub.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX, x > bounds.minX {
                x = bounds.minX
                y += rowHeight + spacing
                rowHeight = 0
            }
            sub.place(at: CGPoint(x: x, y: y), proposal: .unspecified)
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}
