import SwiftUI

struct GitPanel: View {
    @EnvironmentObject private var gitViewModel: GitViewModel
    @EnvironmentObject private var projectViewModel: ProjectViewModel
    @State private var newRevisionName: String = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Revisions")
                    .font(.headline)
                Spacer()
                if gitViewModel.state.isBusy {
                    ProgressView()
                        .scaleEffect(0.6)
                }
            }

            switch gitViewModel.state.repositoryStatus {
            case .missing:
                VStack(alignment: .leading, spacing: 8) {
                    Text("Version history is disabled.")
                        .font(.subheadline)
                    Button("Enable Revisions") {
                        gitViewModel.initializeRepository(projectPath: projectViewModel.project?.url)
                    }
                    .buttonStyle(.borderedProminent)
                }
            case .available:
                if let status = gitViewModel.state.status {
                    branchInfo(status: status)
                    Divider()
                    if status.entries.isEmpty {
                        Text("No unsaved changes.")
                            .foregroundStyle(.secondary)
                    } else {
                        List(status.entries) { entry in
                            HStack {
                                Text(entry.change.displayName)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .frame(width: 70, alignment: .leading)
                                Text(entry.path)
                                    .lineLimit(1)
                            }
                        }
                        .listStyle(.plain)
                        .frame(maxHeight: 140)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        TextField("Write a brief summary…", text: $gitViewModel.commitMessage, axis: .vertical)
                            .textFieldStyle(.roundedBorder)
                        HStack {
                            Button("Save Revision") {
                                gitViewModel.commitChanges(projectPath: projectViewModel.project?.url)
                            }
                            .disabled(!gitViewModel.canCommit)
                            Button("Sync") {
                                gitViewModel.sync(projectPath: projectViewModel.project?.url)
                            }
                            .disabled(!gitViewModel.canSync)
                        }
                    }

                    Divider()
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Alternate drafts")
                            .font(.subheadline)
                        HStack {
                            TextField("New revision name", text: $newRevisionName)
                            Button("Create") {
                                gitViewModel.createRevision(projectPath: projectViewModel.project?.url, name: newRevisionName)
                                newRevisionName = ""
                            }
                            .disabled(newRevisionName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        }
                        if !gitViewModel.state.branches.isEmpty {
                            Picker("Switch Revision", selection: Binding(
                                get: { gitViewModel.state.status?.branch ?? "" },
                                set: { gitViewModel.switchRevision(projectPath: projectViewModel.project?.url, name: $0) }
                            )) {
                                ForEach(gitViewModel.state.branches, id: \.self) { branch in
                                    Text(branch).tag(branch)
                                }
                            }
                            .pickerStyle(.menu)
                        }
                    }
                }
            }

            if let message = gitViewModel.state.message {
                Divider()
                Text(message)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Spacer()
        }
        .padding()
        .frame(minWidth: 280, idealWidth: 300)
        .onAppear {
            gitViewModel.refresh(projectPath: projectViewModel.project?.url)
        }
        .onChange(of: projectViewModel.project?.url) { newValue in
            gitViewModel.refresh(projectPath: newValue)
        }
    }

    @ViewBuilder
    private func branchInfo(status: GitStatus) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            if let branch = status.branch {
                HStack {
                    Label(branch, systemImage: "point.topleft.down.curvedto.point.bottomright.up")
                    if let upstream = status.upstream {
                        Text("↔︎ \(upstream)")
                            .foregroundStyle(.secondary)
                    }
                }
            }
            if status.ahead > 0 || status.behind > 0 {
                Text("\(status.ahead) ahead · \(status.behind) behind")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
    }
}
