import SwiftUI

struct EditorView: View {
    @EnvironmentObject private var projectViewModel: ProjectViewModel
    @StateObject private var controller = RichTextController()
    @State private var currentContent: NSAttributedString = NSAttributedString(string: "")
    private let debouncer = Debouncer(delay: 0.4)

    var body: some View {
        VStack(spacing: 0) {
            EditorToolbar(controller: controller, projectName: projectViewModel.selectedDocument?.name ?? "Document")
            Divider()
            Group {
                if projectViewModel.selectedDocument != nil {
                    RichTextEditor(
                        attributedText: $currentContent,
                        controller: controller,
                        onTextChange: handleTextChange
                    )
                    .background(Color(NSColor.textBackgroundColor))
                } else {
                    VStack {
                        Text("Open a .chikn project to start writing.")
                            .font(.headline)
                        Text("Use File → Open or press ⌘O.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        }
        .onChange(of: projectViewModel.editorContent) { newValue in
            currentContent = newValue
        }
        .onChange(of: projectViewModel.selectedDocumentID) { _ in
            currentContent = projectViewModel.editorContent
        }
        .onAppear {
            currentContent = projectViewModel.editorContent
        }
    }

    private func handleTextChange(_ attributedString: NSAttributedString) {
        projectViewModel.updateEditorContent(attributedString)
        debouncer.schedule {
            projectViewModel.saveCurrentDocument()
        }
    }
}

struct EditorToolbar: View {
    let controller: RichTextController
    let projectName: String
    @EnvironmentObject private var projectViewModel: ProjectViewModel

    var body: some View {
        HStack {
            Text(projectName)
                .font(.headline)
            Spacer()
            Button(action: controller.toggleBold) {
                Image(systemName: "bold")
            }
            .help("Bold")
            Button(action: controller.toggleItalic) {
                Image(systemName: "italic")
            }
            .help("Italic")
            Button(action: controller.toggleUnderline) {
                Image(systemName: "underline")
            }
            .help("Underline")
            Divider()
            Menu {
                Button("Heading 1") { controller.insertHeading(level: 1) }
                Button("Heading 2") { controller.insertHeading(level: 2) }
                Button("Heading 3") { controller.insertHeading(level: 3) }
            } label: {
                Image(systemName: "textformat.size.larger")
            }
            Button(action: controller.insertBulletList) {
                Image(systemName: "list.bullet")
            }
            Divider()
            Button {
                projectViewModel.saveCurrentDocument()
                projectViewModel.saveProject()
            } label: {
                Label("Save", systemImage: "square.and.arrow.down")
            }
            .keyboardShortcut("s", modifiers: .command)
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(.ultraThinMaterial)
    }
}
