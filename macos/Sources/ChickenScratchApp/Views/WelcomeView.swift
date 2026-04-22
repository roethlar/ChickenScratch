import SwiftUI

struct WelcomeView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [.accentColor.opacity(0.25), .accentColor.opacity(0.05)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            GlassEffectContainer(spacing: 16) {
                VStack(spacing: 28) {
                    VStack(spacing: 8) {
                        Text("ChickenScratch")
                            .font(.system(size: 44, weight: .semibold, design: .serif))
                        Text("A writing app for writers.")
                            .font(.title3)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 20)

                    HStack(spacing: 16) {
                        Button {
                            store.openPickedProject()
                        } label: {
                            Label("Open Project…", systemImage: "folder")
                                .font(.body.weight(.medium))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                        }
                        .buttonStyle(.glassProminent)
                        .keyboardShortcut("o")

                        Button {
                            // New project — not yet implemented in the scaffold
                        } label: {
                            Label("New Project…", systemImage: "plus")
                                .font(.body.weight(.medium))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                        }
                        .buttonStyle(.glass)
                        .disabled(true)
                        .help("Coming in a later milestone")
                    }
                }
                .padding(48)
                .panelGlass(cornerRadius: 28)
            }
            .frame(maxWidth: 520)
        }
    }
}
