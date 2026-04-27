using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ChickenScratch.Core.Git;
using ChickenScratch.Core.Models;

namespace ChickenScratch.App.ViewModels;

public partial class RevisionsViewModel : ObservableObject
{
    private string? _projectPath;

    [ObservableProperty] public partial ObservableCollection<Revision> Revisions { get; set; }
    [ObservableProperty] public partial ObservableCollection<DraftVersion> Drafts { get; set; }
    [ObservableProperty] public partial ObservableCollection<FileDiff> SelectedDiffs { get; set; }
    [ObservableProperty] public partial Revision? SelectedRevision { get; set; }
    [ObservableProperty] public partial DraftVersion? SelectedDraft { get; set; }
    [ObservableProperty] public partial bool IsBusy { get; set; }
    [ObservableProperty] public partial string? StatusMessage { get; set; }

    public RevisionsViewModel()
    {
        Revisions = [];
        Drafts = [];
        SelectedDiffs = [];
    }

    public void SetProjectPath(string? path)
    {
        _projectPath = path;
        if (path != null) Refresh();
        else Clear();
    }

    public void Refresh()
    {
        if (_projectPath == null) return;
        try
        {
            Revisions = new ObservableCollection<Revision>(GitService.ListRevisions(_projectPath));
            Drafts = new ObservableCollection<DraftVersion>(GitService.ListDrafts(_projectPath));
        }
        catch
        {
            Revisions = [];
            Drafts = [];
        }
        SelectedRevision = null;
        SelectedDiffs.Clear();
    }

    public void Clear()
    {
        Revisions.Clear();
        Drafts.Clear();
        SelectedDiffs.Clear();
        SelectedRevision = null;
        SelectedDraft = null;
    }

    partial void OnSelectedRevisionChanged(Revision? value)
    {
        SelectedDiffs.Clear();
        if (value == null || _projectPath == null) return;
        try
        {
            foreach (var d in GitService.RevisionDiff(_projectPath, value.Id))
                SelectedDiffs.Add(d);
        }
        catch { /* revision diff failed — leave list empty */ }
    }

    [RelayCommand]
    public async Task RestoreRevisionAsync()
    {
        if (SelectedRevision == null || _projectPath == null) return;
        IsBusy = true;
        var id = SelectedRevision.Id;
        try
        {
            await Task.Run(() => GitService.RestoreRevision(_projectPath, id));
            StatusMessage = "Revision restored";
            Refresh();
        }
        catch (Exception ex) { StatusMessage = $"Restore failed: {ex.Message}"; }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    public async Task NewDraftAsync(string name)
    {
        if (_projectPath == null) return;
        IsBusy = true;
        try
        {
            await Task.Run(() => GitService.CreateDraft(_projectPath, name));
            StatusMessage = $"Created draft \"{name}\"";
            Refresh();
        }
        catch (Exception ex) { StatusMessage = $"Error: {ex.Message}"; }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    public async Task SwitchDraftAsync(string name)
    {
        if (_projectPath == null) return;
        IsBusy = true;
        try
        {
            await Task.Run(() => GitService.SwitchDraft(_projectPath, name));
            StatusMessage = $"Switched to \"{name}\"";
            Refresh();
        }
        catch (Exception ex) { StatusMessage = $"Error: {ex.Message}"; }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    public async Task MergeDraftAsync(string name)
    {
        if (_projectPath == null) return;
        IsBusy = true;
        try
        {
            await Task.Run(() => GitService.MergeDraft(_projectPath, name));
            StatusMessage = $"Merged \"{name}\"";
            Refresh();
        }
        catch (Exception ex) { StatusMessage = $"Error: {ex.Message}"; }
        finally { IsBusy = false; }
    }
}
