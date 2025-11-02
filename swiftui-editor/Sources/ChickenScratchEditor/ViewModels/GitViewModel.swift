import Foundation

@MainActor
final class GitViewModel: ObservableObject {
    @Published private(set) var state = GitPanelState()
    @Published var commitMessage: String = ""
    @Published var showAlert: Bool = false
    @Published var alertMessage: String = ""

    private let service = GitService()

    var canCommit: Bool {
        guard state.repositoryStatus == .available else { return false }
        let hasChanges = !(state.status?.entries.isEmpty ?? true)
        return hasChanges && !commitMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var canSync: Bool {
        guard state.repositoryStatus == .available else { return false }
        return true
    }

    func refresh(projectPath: URL?) {
        guard let projectPath else {
            state = GitPanelState(
                repositoryStatus: .missing,
                status: nil,
                branches: [],
                message: nil,
                isBusy: false
            )
            return
        }

        let status = service.status(at: projectPath)
        let branches = service.listBranches(at: projectPath)
        state = GitPanelState(
            repositoryStatus: status.repositoryStatus,
            status: status,
            branches: branches,
            message: state.message,
            isBusy: false
        )
    }

    func initializeRepository(projectPath: URL?) {
        guard let projectPath else { return }
        state.isBusy = true
        Task {
            let result = service.initializeRepository(at: projectPath)
            await MainActor.run {
                state.message = result.message
                state.isBusy = false
                refresh(projectPath: projectPath)
            }
        }
    }

    func commitChanges(projectPath: URL?) {
        guard let projectPath else { return }
        let message = commitMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !message.isEmpty else { return }

        state.isBusy = true
        Task {
            let result = service.commit(at: projectPath, message: message)
            await MainActor.run {
                state.message = result.message
                state.isBusy = false
                if result.success {
                    commitMessage = ""
                }
                refresh(projectPath: projectPath)
            }
        }
    }

    func sync(projectPath: URL?) {
        guard let projectPath else { return }
        state.isBusy = true
        Task {
            let pullResult = service.pull(at: projectPath)
            let pushResult = service.push(at: projectPath)
            await MainActor.run {
                if !pullResult.success {
                    state.message = pullResult.message
                } else if !pushResult.success {
                    state.message = pushResult.message
                } else {
                    state.message = "Synced revisions."
                }
                state.isBusy = false
                refresh(projectPath: projectPath)
            }
        }
    }

    func createRevision(projectPath: URL?, name: String) {
        guard let projectPath, !name.isEmpty else { return }
        state.isBusy = true
        Task {
            let result = service.createBranch(at: projectPath, name: name)
            await MainActor.run {
                state.message = result.message
                state.isBusy = false
                refresh(projectPath: projectPath)
            }
        }
    }

    func switchRevision(projectPath: URL?, name: String) {
        guard let projectPath else { return }
        state.isBusy = true
        Task {
            let result = service.checkoutBranch(at: projectPath, name: name)
            await MainActor.run {
                state.message = result.message
                state.isBusy = false
                refresh(projectPath: projectPath)
            }
        }
    }
}

struct GitPanelState {
    var repositoryStatus: GitRepositoryStatus = .missing
    var status: GitStatus?
    var branches: [String] = []
    var message: String?
    var isBusy: Bool = false
}
