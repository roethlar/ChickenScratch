using CommunityToolkit.Mvvm.ComponentModel;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;
using Microsoft.UI.Dispatching;

namespace ChickenScratch.App.ViewModels;

public partial class InspectorViewModel : ObservableObject
{
    [ObservableProperty] private string? _synopsis;
    [ObservableProperty] private string? _label;
    [ObservableProperty] private string? _status;
    [ObservableProperty] private bool _includeInCompile = true;
    [ObservableProperty] private uint _wordCountTarget;
    [ObservableProperty] private bool _hasDocument;

    // NumberBox requires double binding
    public double WordCountTargetDouble
    {
        get => WordCountTarget;
        set { WordCountTarget = (uint)Math.Max(0, value); }
    }

    private Document? _document;
    private string? _projectPath;
    private DispatcherQueueTimer? _debounceTimer;
    private bool _loading;

    public void Initialize(DispatcherQueue queue)
    {
        _debounceTimer = queue.CreateTimer();
        _debounceTimer.Interval = TimeSpan.FromMilliseconds(1500);
        _debounceTimer.IsRepeating = false;
        _debounceTimer.Tick += (_, _) => SaveNow();
    }

    public void LoadDocument(Document? doc)
    {
        _debounceTimer?.Stop();
        _document = doc;
        _projectPath = null;
        HasDocument = doc != null;

        if (doc == null) return;

        // Resolve project root: doc.Path is relative like "documents/foo.html"
        // We'll receive the full project path from AppViewModel
        _loading = true;
        Synopsis = doc.Synopsis ?? string.Empty;
        Label = doc.Label ?? string.Empty;
        Status = doc.Status ?? string.Empty;
        IncludeInCompile = doc.IncludeInCompile;
        WordCountTarget = doc.WordCountTarget;
        _loading = false;
    }

    public void SetProjectPath(string projectPath) => _projectPath = projectPath;

    public void Clear()
    {
        _document = null;
        HasDocument = false;
        Synopsis = null;
        Label = null;
        Status = null;
        IncludeInCompile = true;
        WordCountTarget = 0;
    }

    protected override void OnPropertyChanged(System.ComponentModel.PropertyChangedEventArgs e)
    {
        base.OnPropertyChanged(e);
        if (!_loading && _document != null && e.PropertyName is
            nameof(Synopsis) or nameof(Label) or nameof(Status) or
            nameof(IncludeInCompile) or nameof(WordCountTarget))
        {
            _debounceTimer?.Stop();
            _debounceTimer?.Start();
        }
    }

    private void SaveNow()
    {
        if (_document == null || _projectPath == null) return;
        try
        {
            DocumentService.UpdateMetadata(_projectPath, _document.Id, new DocumentMetaUpdate
            {
                Synopsis = Synopsis,
                Label = Label,
                Status = Status,
                IncludeInCompile = IncludeInCompile,
                WordCountTarget = WordCountTarget,
            });
        }
        catch { /* silent */ }
    }
}
