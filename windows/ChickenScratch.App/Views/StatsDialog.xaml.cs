using Microsoft.UI.Xaml.Controls;
using ChickenScratch.Core.Models;

namespace ChickenScratch.App.Views;

public sealed partial class StatsDialog : ContentDialog
{
    private record DocStat(string Name, int Words);

    public StatsDialog(Project project)
    {
        InitializeComponent();

        var entries = project.Documents.Values
            .Select(d => new DocStat(d.Name, CountWords(d.Content)))
            .OrderByDescending(e => e.Words)
            .ToList();

        int total = entries.Sum(e => e.Words);
        int pages  = total / 250;
        int mins   = total / 200;

        TotalWordsText.Text = total.ToString("N0");
        PageCountText.Text  = pages.ToString("N0");
        ReadingMinText.Text  = mins.ToString("N0");
        DocCountText.Text   = entries.Count.ToString("N0");

        DocList.ItemsSource = entries;
    }

    private static int CountWords(string? html)
    {
        if (string.IsNullOrWhiteSpace(html)) return 0;
        // Strip HTML tags before counting
        var text = System.Text.RegularExpressions.Regex.Replace(html, "<[^>]*>", " ");
        return text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Length;
    }
}
