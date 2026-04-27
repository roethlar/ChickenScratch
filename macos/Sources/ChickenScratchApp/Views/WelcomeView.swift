import AppKit
import SwiftUI

struct WelcomeView: View {
    @Environment(ProjectStore.self) private var store
    @State private var showNewProjectSheet = false
    @State private var newProjectName = ""

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [.accentColor.opacity(0.25), .accentColor.opacity(0.05)],
                startPoint: .topLeading, endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            GlassEffectContainer(spacing: 16) {
                VStack(spacing: 28) {
                    VStack(spacing: 8) {
                        Text("ChickenScratch")
                            .font(.system(size: 44, weight: .semibold, design: .serif))
                        Text("A writing app for writers.")
                            .font(.title3).foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 20)

                    HStack(spacing: 16) {
                        Button { store.openPickedProject() } label: {
                            Label("Open Project…", systemImage: "folder")
                                .font(.body.weight(.medium))
                                .padding(.horizontal, 10).padding(.vertical, 6)
                        }
                        .buttonStyle(.glassProminent)
                        .keyboardShortcut("o")

                        Button { showNewProjectSheet = true } label: {
                            Label("New Project…", systemImage: "plus")
                                .font(.body.weight(.medium))
                                .padding(.horizontal, 10).padding(.vertical, 6)
                        }
                        .buttonStyle(.glass)
                        .keyboardShortcut("n")
                    }

                    if !store.recentProjects.isEmpty {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("RECENT")
                                .font(.caption2.weight(.semibold))
                                .foregroundStyle(.secondary)
                            ForEach(store.recentProjects) { recent in
                                Button {
                                    store.open(url: URL(fileURLWithPath: recent.path))
                                } label: {
                                    HStack {
                                        Image(systemName: "doc.text")
                                            .foregroundStyle(.secondary)
                                        VStack(alignment: .leading, spacing: 2) {
                                            Text(recent.name).font(.body.weight(.medium))
                                            Text(recent.path).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                                        }
                                        Spacer()
                                    }
                                    .padding(.horizontal, 12).padding(.vertical, 8)
                                }
                                .buttonStyle(.glass)
                            }
                        }
                        .frame(maxWidth: 400)
                    }
                }
                .padding(48)
                .panelGlass(cornerRadius: 28)
            }
            .frame(maxWidth: 560)
        }
        .sheet(isPresented: $showNewProjectSheet) {
            NewProjectSheet(
                onCommit: { name, url in
                    store.createProject(name: name, at: url)
                    showNewProjectSheet = false
                },
                onCancel: { showNewProjectSheet = false }
            )
        }
    }
}

private struct NewProjectSheet: View {
    let onCommit: (String, URL) -> Void
    let onCancel: () -> Void

    @State private var name: String = ""
    @State private var chosenURL: URL?
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("New Project").font(.headline)

            TextField("Project name", text: $name)
                .textFieldStyle(.roundedBorder)
                .focused($focused)

            HStack {
                Button("Choose folder…") { pickFolder() }
                if let url = chosenURL {
                    Text(url.path).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
            }

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Create") { commit() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || chosenURL == nil)
            }
        }
        .padding(24)
        .frame(width: 400)
        .onAppear { focused = true }
    }

    private func pickFolder() {
        let panel = NSSavePanel()
        panel.title = "Create project folder"
        panel.nameFieldStringValue = name.isEmpty ? "MyProject" : name
        panel.canCreateDirectories = true
        panel.prompt = "Create"
        if panel.runModal() == .OK, let url = panel.url {
            chosenURL = url
            if name.isEmpty { name = url.deletingPathExtension().lastPathComponent }
        }
    }

    private func commit() {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, let url = chosenURL else { return }
        onCommit(trimmed, url)
    }
}
