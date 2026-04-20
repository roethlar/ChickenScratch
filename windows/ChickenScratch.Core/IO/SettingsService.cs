using System.Text.Json;
using System.Text.Json.Serialization;
using ChickenScratch.Core.Models;

namespace ChickenScratch.Core.IO;

public static class SettingsService
{
    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };

    private static string ConfigDir()
    {
        var dir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "chickenscratch");
        Directory.CreateDirectory(dir);
        return dir;
    }

    private static string SettingsPath() => Path.Combine(ConfigDir(), "settings.json");
    private static string RecentPath() => Path.Combine(ConfigDir(), "recent-projects.json");

    public static AppSettings GetSettings()
    {
        var path = SettingsPath();
        if (!File.Exists(path)) return new AppSettings();
        try
        {
            var json = File.ReadAllText(path);
            return JsonSerializer.Deserialize<AppSettings>(json, JsonOpts) ?? new AppSettings();
        }
        catch { return new AppSettings(); }
    }

    public static void SaveSettings(AppSettings settings)
    {
        var json = JsonSerializer.Serialize(settings, JsonOpts);
        File.WriteAllText(SettingsPath(), json);
    }

    public static List<RecentProject> GetRecentProjects()
    {
        var path = RecentPath();
        if (!File.Exists(path)) return [];
        try
        {
            var json = File.ReadAllText(path);
            return JsonSerializer.Deserialize<List<RecentProject>>(json, JsonOpts) ?? [];
        }
        catch { return []; }
    }

    public static void AddRecentProject(string name, string path)
    {
        var settings = GetSettings();
        var recent = GetRecentProjects();
        recent.RemoveAll(r => r.Path == path);
        recent.Insert(0, new RecentProject { Name = name, Path = path });
        if (recent.Count > settings.General.RecentProjectsLimit)
            recent = recent[..settings.General.RecentProjectsLimit];

        var json = JsonSerializer.Serialize(recent, JsonOpts);
        File.WriteAllText(RecentPath(), json);
    }
}
