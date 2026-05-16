using ChickenScratch.Core.IO;

if (args.Length != 1)
{
    Console.Error.WriteLine("usage: dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness <project.chikn>");
    return 1;
}

var projectPath = Path.GetFullPath(args[0]);
var project = ProjectReader.ReadProject(projectPath);
var doc = project.Documents.Values.OrderBy(d => d.Path, StringComparer.Ordinal).FirstOrDefault();

if (doc == null)
{
    Console.Error.WriteLine("ChickenScratch.Core.CrossFrontendHarness: project has no documents");
    return 1;
}

doc.Synopsis = "Cross-frontend harness: C# writer pass";
doc.Fields["cross_frontend_csharp"] = "ran";
doc.Fields["cross_frontend_sequence"] = new[] { "rust-converter", "swift-chiknkit", "csharp-core" };

ProjectWriter.WriteProject(project);

Console.WriteLine($"csharp: wrote {doc.Path} in {project.Path}");
return 0;
