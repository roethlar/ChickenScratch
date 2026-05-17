import Foundation
import Darwin

enum SafeProjectPath {
    static func documentURLs(
        projectURL: URL,
        relativePath: String,
        createParentDirectories: Bool
    ) throws -> (document: URL, meta: URL) {
        let components = try validate(relativePath)
        let projectRoot = try canonicalProjectRoot(projectURL)
        let parentComponents = Array(components.dropLast())
        let parentURL = try ensureParentDirectory(
            projectURL: projectURL,
            projectRoot: projectRoot,
            components: parentComponents,
            documentPath: relativePath,
            createMissing: createParentDirectories
        )

        guard let fileName = components.last else {
            throw invalid(relativePath, "path must contain a file name")
        }

        let documentURL = parentURL.appendingPathComponent(fileName, isDirectory: false)
        try ensureExistingTargetSafe(
            documentURL,
            projectRoot: projectRoot,
            documentPath: relativePath,
            kind: "document file"
        )

        let metaURL = documentURL.deletingPathExtension().appendingPathExtension("meta")
        try ensureExistingTargetSafe(
            metaURL,
            projectRoot: projectRoot,
            documentPath: relativePath,
            kind: "document metadata"
        )

        return (documentURL, metaURL)
    }

    static func relativeDocumentPath(for fileURL: URL, in projectURL: URL) throws -> String? {
        let projectRoot = try canonicalProjectRoot(projectURL)
        let filePath = fileURL.standardizedFileURL.resolvingSymlinksInPath().path
        let rootPath = projectRoot.path
        let prefix = rootPath.hasSuffix("/") ? rootPath : rootPath + "/"
        guard filePath.hasPrefix(prefix) else { return nil }

        let relativePath = String(filePath.dropFirst(prefix.count))
        _ = try documentURLs(
            projectURL: projectURL,
            relativePath: relativePath,
            createParentDirectories: false
        )
        return relativePath
    }

    private static func validate(_ relativePath: String) throws -> [String] {
        guard !relativePath.isEmpty else {
            throw invalid(relativePath, "path must contain a file name")
        }
        if relativePath.hasPrefix("/") {
            throw invalid(relativePath, "absolute paths are not allowed")
        }

        let components = relativePath.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
        guard !components.isEmpty else {
            throw invalid(relativePath, "path must contain a file name")
        }
        for component in components {
            if component == "." {
                throw invalid(relativePath, "current-directory components are not allowed")
            }
            if component == ".." {
                throw invalid(relativePath, "parent-directory components are not allowed")
            }
        }
        return components
    }

    private static func canonicalProjectRoot(_ projectURL: URL) throws -> URL {
        let root = projectURL.standardizedFileURL.resolvingSymlinksInPath()
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: root.path, isDirectory: &isDirectory), isDirectory.boolValue else {
            throw ChiknError.invalidDocumentPath("Project path is not a directory: \(projectURL.path)")
        }
        return root
    }

    private static func ensureParentDirectory(
        projectURL: URL,
        projectRoot: URL,
        components: [String],
        documentPath: String,
        createMissing: Bool
    ) throws -> URL {
        var current = projectURL.standardizedFileURL

        for component in components {
            current.appendPathComponent(component, isDirectory: true)

            if try isSymlink(current) {
                throw invalid(documentPath, "path traverses a symlink: \(current.path)")
            }

            var isDirectory: ObjCBool = false
            if FileManager.default.fileExists(atPath: current.path, isDirectory: &isDirectory) {
                try ensureDirectorySafe(
                    current,
                    projectRoot: projectRoot,
                    documentPath: documentPath,
                    isDirectory: isDirectory.boolValue
                )
                continue
            }

            guard createMissing else { return current }
            try FileManager.default.createDirectory(at: current, withIntermediateDirectories: false)
            try ensureDirectorySafe(
                current,
                projectRoot: projectRoot,
                documentPath: documentPath,
                isDirectory: true
            )
        }

        return current
    }

    private static func ensureDirectorySafe(
        _ url: URL,
        projectRoot: URL,
        documentPath: String,
        isDirectory: Bool
    ) throws {
        if try isSymlink(url) {
            throw invalid(documentPath, "path traverses a symlink: \(url.path)")
        }
        guard isDirectory else {
            throw invalid(documentPath, "parent is not a directory: \(url.path)")
        }
        try ensureWithinProject(url, projectRoot: projectRoot, documentPath: documentPath)
    }

    private static func ensureExistingTargetSafe(
        _ url: URL,
        projectRoot: URL,
        documentPath: String,
        kind: String
    ) throws {
        if try isSymlink(url) {
            throw invalid(documentPath, "\(kind) is a symlink: \(url.path)")
        }
        guard FileManager.default.fileExists(atPath: url.path) else { return }
        try ensureWithinProject(url, projectRoot: projectRoot, documentPath: documentPath)
    }

    private static func ensureWithinProject(_ url: URL, projectRoot: URL, documentPath: String) throws {
        let path = url.standardizedFileURL.resolvingSymlinksInPath().path
        let root = projectRoot.path
        let prefix = root.hasSuffix("/") ? root : root + "/"
        guard path == root || path.hasPrefix(prefix) else {
            throw invalid(documentPath, "path escapes project root: \(url.path)")
        }
    }

    private static func isSymlink(_ url: URL) throws -> Bool {
        var info = stat()
        let result = url.path.withCString { lstat($0, &info) }
        if result == 0 {
            return (info.st_mode & S_IFMT) == S_IFLNK
        }
        if errno == ENOENT || errno == ENOTDIR {
            return false
        }
        throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
    }

    private static func invalid(_ documentPath: String, _ reason: String) -> ChiknError {
        ChiknError.invalidDocumentPath(
            "Document path must be relative and within project (\(reason)): \(documentPath)"
        )
    }
}
