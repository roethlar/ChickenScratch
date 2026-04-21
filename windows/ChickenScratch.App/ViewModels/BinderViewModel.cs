using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;

namespace ChickenScratch.App.ViewModels;

public partial class BinderItemViewModel : ObservableObject
{
    [ObservableProperty] public partial string Name { get; set; }
    [ObservableProperty] public partial bool IsEditing { get; set; }
    [ObservableProperty] public partial string EditName { get; set; }

    public string Id { get; }
    public bool IsDocument { get; }
    public bool IsFolder => !IsDocument;
    public ObservableCollection<BinderItemViewModel> Children { get; } = [];

    public BinderItemViewModel(string id, string name, bool isDocument)
    {
        Id = id;
        Name = name;
        EditName = name;
        IsDocument = isDocument;
    }
}

public partial class BinderViewModel : ObservableObject
{
    [ObservableProperty] public partial ObservableCollection<BinderItemViewModel> Nodes { get; set; }
    [ObservableProperty] public partial BinderItemViewModel? SelectedItem { get; set; }

    private string? _projectPath;

    public event Action<Project>? ProjectChanged;

    public BinderViewModel()
    {
        Nodes = [];
    }

    public void LoadProject(Project project)
    {
        _projectPath = project.Path;
        Nodes = BuildTree(project.Hierarchy);
    }

    public void Clear()
    {
        _projectPath = null;
        Nodes = [];
        SelectedItem = null;
    }

    public void Refresh(Project project)
    {
        LoadProject(project);
        ProjectChanged?.Invoke(project);
    }

    private static ObservableCollection<BinderItemViewModel> BuildTree(List<TreeNode> nodes)
    {
        var result = new ObservableCollection<BinderItemViewModel>();
        foreach (var node in nodes)
            result.Add(BuildItem(node));
        return result;
    }

    private static BinderItemViewModel BuildItem(TreeNode node)
    {
        var vm = new BinderItemViewModel(node.Id, node.Name, node is DocumentNode);
        if (node is FolderNode folder)
            foreach (var child in folder.Children)
                vm.Children.Add(BuildItem(child));
        return vm;
    }

    [RelayCommand]
    public void NewDocument(string? parentId = null)
    {
        if (_projectPath == null) return;
        var project = DocumentService.CreateDocument(_projectPath, "New Document", parentId);
        Refresh(project);
    }

    [RelayCommand]
    public void NewFolder(string? parentId = null)
    {
        if (_projectPath == null) return;
        var project = DocumentService.CreateFolder(_projectPath, "New Folder", parentId);
        Refresh(project);
    }

    [RelayCommand]
    public void DeleteSelected()
    {
        if (_projectPath == null || SelectedItem == null) return;
        var project = DocumentService.DeleteNode(_projectPath, SelectedItem.Id);
        Refresh(project);
    }

    [RelayCommand]
    public void RenameSelected(string newName)
    {
        if (_projectPath == null || SelectedItem == null) return;
        var project = DocumentService.RenameNode(_projectPath, SelectedItem.Id, newName);
        Refresh(project);
    }

    [RelayCommand]
    public void MoveSelectedNode(string newParentId)
    {
        if (_projectPath == null || SelectedItem == null) return;
        var project = DocumentService.MoveNode(_projectPath, SelectedItem.Id, newParentId);
        Refresh(project);
    }
}
