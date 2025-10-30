import SwiftUI

struct ContentView: View {

    var body: some View {
        HStack(spacing: 0) {
            NavigatorView()
            Divider()
            EditorView()
                .frame(minWidth: 700)
            Divider()
            VStack(spacing: 0) {
                ScrollView {
                    InspectorView()
                }
                Divider()
                GitPanel()
            }
            .frame(width: 320)
        }
        .frame(minWidth: 1200, minHeight: 720)
    }
}
