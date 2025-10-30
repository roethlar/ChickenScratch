import SwiftUI

@main
struct ChickenScratchEditorApp: App {
    @StateObject private var projectViewModel = ProjectViewModel()
    @StateObject private var gitViewModel = GitViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(projectViewModel)
                .environmentObject(gitViewModel)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open .chikn Project…") {
                    projectViewModel.presentOpenPanel()
                }
                .keyboardShortcut("o", modifiers: [.command])
            }
            CommandMenu("Revisions") {
                Button("Initialize Revisions") {
                    gitViewModel.initializeRepository(projectPath: projectViewModel.project?.url)
                }
                .disabled(!(projectViewModel.project?.url != nil && gitViewModel.state.repositoryStatus == .missing))

                Divider()

                Button("Save Revision…") {
                    gitViewModel.commitChanges(projectPath: projectViewModel.project?.url)
                }
                .disabled(!gitViewModel.canCommit)

                Button("Sync Revisions") {
                    gitViewModel.sync(projectPath: projectViewModel.project?.url)
                }
                .disabled(!gitViewModel.canSync)
            }
        }
    }
}
