import SwiftUI

struct RootView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        Group {
            if store.project == nil {
                WelcomeView()
            } else {
                ProjectWindow()
            }
        }
        .alert(
            "Couldn't open project",
            isPresented: Binding(
                get: { store.errorMessage != nil },
                set: { if !$0 { store.errorMessage = nil } }
            ),
            actions: { Button("OK", role: .cancel) { store.errorMessage = nil } },
            message: { Text(store.errorMessage ?? "") }
        )
    }
}
