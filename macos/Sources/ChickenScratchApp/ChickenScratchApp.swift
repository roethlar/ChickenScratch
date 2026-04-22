import SwiftUI

@main
struct ChickenScratchApp: App {
    @State private var store = ProjectStore()

    var body: some Scene {
        WindowGroup("ChickenScratch") {
            RootView()
                .environment(store)
                .frame(minWidth: 900, minHeight: 600)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open Project…") { store.openPickedProject() }
                    .keyboardShortcut("o")
                Button("Close Project") { store.closeProject() }
                    .keyboardShortcut("w", modifiers: [.command, .shift])
                    .disabled(store.project == nil)
            }
        }
    }
}
