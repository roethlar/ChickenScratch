import SwiftUI

struct NavigatorView: View {
    @EnvironmentObject private var projectViewModel: ProjectViewModel
    @EnvironmentObject private var gitViewModel: GitViewModel

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Navigator")
                    .font(.headline)
                Spacer()
                Button {
                    if let url = projectViewModel.project?.url {
                        gitViewModel.refresh(projectPath: url)
                    }
                } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .buttonStyle(.plain)
                .help("Refresh project information")
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            Divider()
            List {
                if let hierarchy = projectViewModel.project?.hierarchy {
                    OutlineGroup(hierarchy, children: \.children) { node in
                        NavigatorRow(
                            node: node,
                            isSelected: node.id == projectViewModel.selectedDocumentID
                        )
                        .contentShape(Rectangle())
                        .onTapGesture {
                            projectViewModel.select(node: node)
                        }
                    }
                } else {
                    Text("Open a project to view documents.")
                        .foregroundStyle(.secondary)
                }
            }
            .listStyle(.sidebar)
        }
        .frame(minWidth: 240, idealWidth: 260, maxWidth: 320)
    }

}

private struct NavigatorRow: View {
    let node: ChiknTreeNode
    let isSelected: Bool

    var body: some View {
        HStack {
            Image(systemName: iconName)
                .foregroundStyle(iconColor)
            Text(node.name)
                .foregroundStyle(isSelected ? Color.accentColor : Color.primary)
            Spacer()
        }
        .padding(.vertical, 4)
    }

    private var iconName: String {
        switch node {
        case .folder:
            return "folder"
        case .document:
            return "doc.text"
        }
    }

    private var iconColor: Color {
        switch node {
        case .folder:
            return .yellow
        case .document:
            return .blue
        }
    }
}
