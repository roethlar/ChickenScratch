using CommunityToolkit.Mvvm.ComponentModel;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;
using Microsoft.UI.Dispatching;

namespace ChickenScratch.App.ViewModels;

public partial class InspectorViewModel : ObservableObject
{
    [ObservableProperty] public partial string? Synopsis { get; set; }
    [ObservableProperty] public partial string? Label { get; set; }
    [ObservableProperty] public partial string? Status { get; set; }
    [ObservableProperty] public partial bool IncludeInCompile { get; set; }
    [ObservableProperty] public partial uint WordCountTarget { get; set; }
    [ObservableProperty] public partial bool HasDocument { get; set; }

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

    public InspectorViewModel()
    {
        IncludeInCompile = true;
    }

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
