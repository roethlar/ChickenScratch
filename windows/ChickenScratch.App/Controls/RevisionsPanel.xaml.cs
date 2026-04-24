using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using ChickenScratch.App.ViewModels;
using ChickenScratch.Core.Models;

namespace ChickenScratch.App.Controls;

public sealed partial class RevisionsPanel : UserControl
{
    public static readonly DependencyProperty ViewModelProperty =
        DependencyProperty.Register(nameof(ViewModel), typeof(RevisionsViewModel),
            typeof(RevisionsPanel), new PropertyMetadata(null));

    public RevisionsViewModel ViewModel
    {
        get => (RevisionsViewModel)GetValue(ViewModelProperty);
        set => SetValue(ViewModelProperty, value);
    }

    public RevisionsPanel() => InitializeComponent();

    private void Revisions_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ViewModel != null && sender is ListView lv)
            ViewModel.SelectedRevision = lv.SelectedItem as Revision;
    }

    private void Drafts_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ViewModel != null && sender is ListView lv)
            ViewModel.SelectedDraft = lv.SelectedItem as DraftVersion;
    }

    private async void Restore_Click(object sender, RoutedEventArgs e)
    {
        if (ViewModel?.SelectedRevision == null) return;

        var dialog = new ContentDialog
        {
            Title = "Restore Revision",
            Content = $"Restore to \"{ViewModel.SelectedRevision.Message}\"?\n\nThis creates a new commit restoring that state.",
            PrimaryButtonText = "Restore",
            CloseButtonText = "Cancel",
            XamlRoot = XamlRoot,
            DefaultButton = ContentDialogButton.Primary,
        };

        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            await ViewModel.RestoreRevisionAsync();
    }

    private async void NewDraft_Click(object sender, RoutedEventArgs e)
    {
        if (ViewModel == null) return;

        var box = new TextBox { PlaceholderText = "Draft name…", MinWidth = 240 };
        var dialog = new ContentDialog
        {
            Title = "New Draft",
            Content = box,
            PrimaryButtonText = "Create",
            CloseButtonText = "Cancel",
            XamlRoot = XamlRoot,
            DefaultButton = ContentDialogButton.Primary,
        };

        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
        {
            var name = box.Text.Trim();
            if (!string.IsNullOrEmpty(name))
                await ViewModel.NewDraftAsync(name);
        }
    }

    private async void SwitchDraft_Click(object sender, RoutedEventArgs e)
    {
        if (ViewModel?.SelectedDraft == null) return;
        await ViewModel.SwitchDraftAsync(ViewModel.SelectedDraft.Name);
    }

    private async void MergeDraft_Click(object sender, RoutedEventArgs e)
    {
        if (ViewModel?.SelectedDraft == null) return;
        var name = ViewModel.SelectedDraft.Name;

        var dialog = new ContentDialog
        {
            Title = "Merge Draft",
            Content = $"Merge draft \"{name}\" into the current branch?",
            PrimaryButtonText = "Merge",
            CloseButtonText = "Cancel",
            XamlRoot = XamlRoot,
            DefaultButton = ContentDialogButton.Primary,
        };

        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            await ViewModel.MergeDraftAsync(name);
    }
}
