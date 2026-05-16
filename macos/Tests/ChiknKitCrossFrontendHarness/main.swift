import Foundation
import ChiknKit

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("ChiknKitCrossFrontendHarness: \(message)\n".utf8))
    exit(1)
}

guard CommandLine.arguments.count == 2 else {
    fail("usage: swift run ChiknKitCrossFrontendHarness <project.chikn>")
}

let projectURL = URL(fileURLWithPath: CommandLine.arguments[1]).standardizedFileURL
var project = try Reader.readProject(at: projectURL)

guard let doc = project.documents.values.sorted(by: { $0.relativePath < $1.relativePath }).first else {
    fail("project has no documents")
}

var meta = doc.meta
meta.synopsis = "Cross-frontend harness: Swift writer pass"
meta.fields["cross_frontend_swift"] = .string("ran")
meta.fields["cross_frontend_sequence"] = .array([
    .string("rust-converter"),
    .string("swift-chiknkit"),
])

project = try Writer.saveDocumentMeta(id: doc.id, meta: meta, in: project)

print("swift: wrote \(doc.relativePath) in \(project.path.path)")
