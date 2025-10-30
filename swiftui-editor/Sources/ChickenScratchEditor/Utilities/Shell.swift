import Foundation

struct ShellResult {
    let status: Int32
    let stdout: String
    let stderr: String

    var succeeded: Bool {
        status == 0
    }
}

enum Shell {
    static func run(
        _ command: [String],
        workingDirectory: URL
    ) -> ShellResult {
        let process = Process()
        process.currentDirectoryURL = workingDirectory
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = command

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        do {
            try process.run()
        } catch {
            return ShellResult(status: -1, stdout: "", stderr: error.localizedDescription)
        }

        process.waitUntilExit()

        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
        let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
        let stderr = String(data: stderrData, encoding: .utf8) ?? ""

        return ShellResult(status: process.terminationStatus, stdout: stdout, stderr: stderr)
    }
}
