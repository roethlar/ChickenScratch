using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using ChickenScratch.App.ViewModels;

namespace ChickenScratch.App.Controls;

public sealed partial class BinderControl : UserControl
{
    public static readonly DependencyProperty ViewModelProperty =
        DependencyProperty.Register(nameof(ViewModel), typeof(BinderViewModel),
            typeof(BinderControl), new PropertyMetadata(null));

    public BinderViewModel ViewModel
    {
        get => (BinderViewModel)GetValue(ViewModelProperty);
        set => SetValue(ViewModelProperty, value);
    }

    public event EventHandler<string>? DocumentSelected;

    public BinderControl() => InitializeComponent();

    private void Tree_ItemInvoked(TreeView sender, TreeViewItemInvokedEventArgs args)
    {
        if (args.InvokedItem is BinderItemViewModel item && item.IsDocument)
            DocumentSelected?.Invoke(this, item.Id);
    }

    private void NewDocument_Click(object sender, RoutedEventArgs e)
    {
        var parentId = (BinderTree.SelectedItem as BinderItemViewModel)?.Id;
        ViewModel?.NewDocumentCommand.Execute(parentId);
    }

    private void NewFolder_Click(object sender, RoutedEventArgs e)
    {
        var parentId = (BinderTree.SelectedItem as BinderItemViewModel)?.Id;
        ViewModel?.NewFolderCommand.Execute(parentId);
    }

    private void Delete_Click(object sender, RoutedEventArgs e) =>
        ViewModel?.DeleteSelectedCommand.Execute(null);

    private void ContextNewDoc_Click(object sender, RoutedEventArgs e)
    {
        var id = (sender as FrameworkElement)?.Tag as string;
        ViewModel?.NewDocumentCommand.Execute(id);
    }

    private void ContextNewFolder_Click(object sender, RoutedEventArgs e)
    {
        var id = (sender as FrameworkElement)?.Tag as string;
        ViewModel?.NewFolderCommand.Execute(id);
    }

    private void ContextDelete_Click(object sender, RoutedEventArgs e) =>
        ViewModel?.DeleteSelectedCommand.Execute(null);

    private void ContextRename_Click(object sender, RoutedEventArgs e)
    {
        if (ViewModel?.SelectedItem is BinderItemViewModel item)
            BeginRename(item);
    }

    private void BeginRename(BinderItemViewModel item)
    {
        item.EditName = item.Name;
        item.IsEditing = true;
    }

    private void CommitRename(BinderItemViewModel item)
    {
        item.IsEditing = false;
        if (!string.IsNullOrWhiteSpace(item.EditName) && item.EditName != item.Name)
            ViewModel?.RenameSelectedCommand.Execute(item.EditName);
    }

    private void RenameBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (sender is TextBox tb && tb.Tag is string id)
        {
            var item = FindItem(ViewModel?.Nodes, id);
            if (item == null) return;

            if (e.Key == Windows.System.VirtualKey.Enter) { CommitRename(item); e.Handled = true; }
            else if (e.Key == Windows.System.VirtualKey.Escape) { item.IsEditing = false; e.Handled = true; }
        }
    }

    private void RenameBox_LostFocus(object sender, RoutedEventArgs e)
    {
        if (sender is TextBox tb && tb.Tag is string id)
        {
            var item = FindItem(ViewModel?.Nodes, id);
            if (item?.IsEditing == true) CommitRename(item);
        }
    }

    private static BinderItemViewModel? FindItem(
        System.Collections.ObjectModel.ObservableCollection<BinderItemViewModel>? nodes, string id)
    {
        if (nodes == null) return null;
        foreach (var n in nodes)
        {
            if (n.Id == id) return n;
            var found = FindItem(n.Children, id);
            if (found != null) return found;
        }
        return null;
    }
}
