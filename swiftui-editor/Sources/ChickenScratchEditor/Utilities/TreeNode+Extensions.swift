import Foundation

extension ChiknTreeNode {
    var children: [ChiknTreeNode]? {
        switch self {
        case .folder(let folder):
            return folder.children
        case .document:
            return nil
        }
    }
}
