using Microsoft.UI.Xaml;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;

namespace ChickenScratch.App.ViewModels;

public enum ViewMode { Editor, Preview }

public partial class AppViewModel : ObservableObject
{
    [ObservableProperty] public partial Project? CurrentProject { get; set; }
    [ObservableProperty] public partial Document? ActiveDocument { get; set; }
    [ObservableProperty] public partial bool ShowBinder { get; set; }
    [ObservableProperty] public partial bool ShowInspector { get; set; }
    [ObservableProperty] public partial bool ShowRevisions { get; set; }
    [ObservableProperty] public partial ViewMode CurrentView { get; set; }
    [ObservableProperty] public partial string? StatusMessage { get; set; }
    [ObservableProperty] public partial bool IsBusy { get; set; }

    public bool IsProjectOpen => CurrentProject != null;
    public string ProjectTitle => CurrentProject?.Name ?? string.Empty;

    public Visibility WelcomeVisibility => IsProjectOpen ? Visibility.Collapsed : Visibility.Visible;
    public Visibility EditorVisibility  => IsProjectOpen ? Visibility.Visible  : Visibility.Collapsed;

    public BinderViewModel Binder { get; } = new();
    public EditorViewModel Editor { get; } = new();
    public InspectorViewModel Inspector { get; } = new();

    public AppViewModel()
    {
        ShowBinder = true;
        ShowInspector = true;
        Binder.ProjectChanged += p => CurrentProject = p;
    }

    partial void OnCurrentProjectChanged(Project? value)
    {
        OnPropertyChanged(nameof(IsProjectOpen));
        OnPropertyChanged(nameof(ProjectTitle));
        OnPropertyChanged(nameof(WelcomeVisibility));
        OnPropertyChanged(nameof(EditorVisibility));
        if (value != null)
        {
            Binder.LoadProject(value);
            SettingsService.AddRecentProject(value.Name, value.Path);
        }
    }

    partial void OnActiveDocumentChanged(Document? value)
    {
        Editor.LoadDocument(value);
        Inspector.LoadDocument(value);
    }

    [RelayCommand]
    public async Task OpenProjectAsync(string path)
    {
        IsBusy = true;
        try
        {
            var project = await Task.Run(() => ProjectReader.ReadProject(path));
            CurrentProject = project;
            StatusMessage = $"Opened {project.Name}";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Error: {ex.Message}";
        }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    public async Task CreateProjectAsync((string name, string path) args)
    {
        IsBusy = true;
        try
        {
            var project = await Task.Run(() => ProjectWriter.CreateProject(args.path, args.name));
            CurrentProject = project;
            StatusMessage = $"Created {project.Name}";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Error: {ex.Message}";
        }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    public void CloseProject()
    {
        ActiveDocument = null;
        CurrentProject = null;
        Binder.Clear();
        Editor.Clear();
        Inspector.Clear();
    }

    [RelayCommand]
    public async Task SaveRevisionAsync(string message)
    {
        if (CurrentProject == null) return;
        var path = CurrentProject.Path;
        await Task.Run(() => Core.Git.GitService.SaveRevision(path, message));
        StatusMessage = "Revision saved";
    }

    public void SelectDocument(string docId)
    {
        if (CurrentProject?.Documents.TryGetValue(docId, out var doc) == true)
            ActiveDocument = doc;
    }

    public void RefreshProject()
    {
        if (CurrentProject == null) return;
        CurrentProject = ProjectReader.ReadProject(CurrentProject.Path);
    }
}
