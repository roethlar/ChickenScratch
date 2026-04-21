using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;
using Microsoft.UI.Dispatching;

namespace ChickenScratch.App.ViewModels;

public enum SaveStatus { Saved, Modified, Saving }

public partial class EditorViewModel : ObservableObject
{
    [ObservableProperty] public partial string HtmlContent { get; set; }
    [ObservableProperty] public partial int WordCount { get; set; }
    [ObservableProperty] public partial int SessionWordCount { get; set; }
    [ObservableProperty] public partial SaveStatus SaveStatus { get; set; }
    [ObservableProperty] public partial bool IsBold { get; set; }
    [ObservableProperty] public partial bool IsItalic { get; set; }
    [ObservableProperty] public partial bool IsUnderline { get; set; }
    [ObservableProperty] public partial bool IsEditable { get; set; }

    public string WordCountText => $"{WordCount:N0} words";
    public string SaveStatusText => SaveStatus switch
    {
        SaveStatus.Saved    => "Saved",
        SaveStatus.Modified => "Modified",
        SaveStatus.Saving   => "Saving\u2026",
        _ => string.Empty,
    };

    partial void OnWordCountChanged(int value)   => OnPropertyChanged(nameof(WordCountText));
    partial void OnSaveStatusChanged(SaveStatus value) => OnPropertyChanged(nameof(SaveStatusText));

    private Document? _document;
    private string? _projectPath;
    private DispatcherQueueTimer? _autoSaveTimer;
    private int _sessionStartWords;

    public EditorViewModel()
    {
        HtmlContent = string.Empty;
        IsEditable = true;
    }

    public void Initialize(DispatcherQueue queue)
    {
        _autoSaveTimer = queue.CreateTimer();
        _autoSaveTimer.Interval = TimeSpan.FromSeconds(2);
        _autoSaveTimer.IsRepeating = false;
        _autoSaveTimer.Tick += (_, _) => SaveNow();
    }

    public void SetProjectPath(string projectPath) => _projectPath = projectPath;

    public void LoadDocument(Document? doc)
    {
        _autoSaveTimer?.Stop();
        _document = doc;

        if (doc != null)
        {
            HtmlContent = doc.Content;
            var wc = CountWords(doc.Content);
            WordCount = wc;
            _sessionStartWords = wc;
            SessionWordCount = 0;
            SaveStatus = SaveStatus.Saved;
            IsEditable = true;
        }
        else
        {
            Clear();
        }
    }

    public void Clear()
    {
        _document = null;
        _projectPath = null;
        HtmlContent = string.Empty;
        WordCount = 0;
        SessionWordCount = 0;
        SaveStatus = SaveStatus.Saved;
    }

    public void OnContentChanged(string html, int wordCount)
    {
        if (_document == null) return;
        HtmlContent = html;
        WordCount = wordCount;
        SessionWordCount = Math.Max(0, wordCount - _sessionStartWords);
        SaveStatus = SaveStatus.Modified;
        _autoSaveTimer?.Stop();
        _autoSaveTimer?.Start();
    }

    public void OnSelectionFormatChanged(bool bold, bool italic, bool underline)
    {
        IsBold = bold;
        IsItalic = italic;
        IsUnderline = underline;
    }

    private void SaveNow()
    {
        if (_document == null || _projectPath == null) return;
        SaveStatus = SaveStatus.Saving;
        try
        {
            _document.Content = HtmlContent;
            DocumentService.UpdateContent(
                _projectPath,
                _document.Id,
                HtmlContent);
            SaveStatus = SaveStatus.Saved;
        }
        catch { SaveStatus = SaveStatus.Modified; }
    }

    private static int CountWords(string html)
    {
        var text = System.Text.RegularExpressions.Regex.Replace(html, "<[^>]+>", " ");
        return text.Split([' ', '\t', '\n', '\r'], StringSplitOptions.RemoveEmptyEntries).Length;
    }
}
