using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Storage.Pickers;
using WinRT.Interop;
using ChickenScratch.Core.Compile;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;
using ChickenScratch.Core.Scrivener;
using WinUIEx;

namespace ChickenScratch.App.Views;

public sealed partial class WelcomePage : UserControl
{
    public event EventHandler<string>? ProjectOpened;
    public event EventHandler<(string name, string path)>? ProjectCreated;

    public WelcomePage()
    {
        InitializeComponent();
        Loaded += OnLoaded;
    }

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        // Check pandoc
        await Task.Run(() =>
        {
            var found = PandocService.FindPandoc() != null;
            DispatcherQueue.TryEnqueue(() => PandocWarning.IsOpen = !found);
        });

        // Load recent projects
        var recent = await Task.Run(SettingsService.GetRecentProjects);
        RecentPanel.Visibility = recent.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
        RecentList.ItemsSource = recent;
    }

    private async void OpenProject_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker();
        picker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        picker.FileTypeFilter.Add("*");

        var hwnd = WindowNative.GetWindowHandle(App.MainWindow!);
        WinRT.Interop.InitializeWithWindow.Initialize(picker, hwnd);

        var folder = await picker.PickSingleFolderAsync();
        if (folder != null)
            ProjectOpened?.Invoke(this, folder.Path);
    }

    private async void NewProject_Click(object sender, RoutedEventArgs e)
    {
        var nameBox = new TextBox { PlaceholderText = "My Novel", MinWidth = 300 };
        var dialog = new ContentDialog
        {
            Title = "New Project",
            Content = new StackPanel
            {
                Spacing = 8,
                Children =
                {
                    new TextBlock { Text = "Project name:" },
                    nameBox,
                }
            },
            PrimaryButtonText = "Create",
            CloseButtonText = "Cancel",
            XamlRoot = XamlRoot,
            DefaultButton = ContentDialogButton.Primary,
        };

        if (await dialog.ShowAsync() != ContentDialogResult.Primary) return;

        var name = nameBox.Text.Trim();
        if (string.IsNullOrEmpty(name)) name = "Untitled";

        var savePicker = new FileSavePicker();
        savePicker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        savePicker.SuggestedFileName = name + ".chikn";
        savePicker.FileTypeChoices.Add("ChickenScratch Project", [".chikn"]);

        var hwnd = WindowNative.GetWindowHandle(App.MainWindow!);
        WinRT.Interop.InitializeWithWindow.Initialize(savePicker, hwnd);

        var file = await savePicker.PickSaveFileAsync();
        if (file == null) return;

        var dir = Path.GetDirectoryName(file.Path)!;
        var projectPath = Path.Combine(dir, name + ".chikn");
        ProjectCreated?.Invoke(this, (name, projectPath));
    }

    private async void ImportScrivener_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker();
        picker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        picker.FileTypeFilter.Add("*");

        var hwnd = WindowNative.GetWindowHandle(App.MainWindow!);
        WinRT.Interop.InitializeWithWindow.Initialize(picker, hwnd);

        var folder = await picker.PickSingleFolderAsync();
        if (folder == null) return;

        var scrivPath = folder.Path;
        var defaultName = Path.GetFileNameWithoutExtension(scrivPath);

        var savePicker = new FileSavePicker();
        savePicker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        savePicker.SuggestedFileName = defaultName + ".chikn";
        savePicker.FileTypeChoices.Add("ChickenScratch Project", [".chikn"]);
        WinRT.Interop.InitializeWithWindow.Initialize(savePicker, hwnd);

        var outFile = await savePicker.PickSaveFileAsync();
        if (outFile == null) return;

        var outputPath = outFile.Path;
        ErrorBar.IsOpen = false;

        try
        {
            var project = await Task.Run(() => ScrivenerImporter.Import(scrivPath, outputPath));
            ProjectOpened?.Invoke(this, outputPath);
        }
        catch (Exception ex)
        {
            ErrorBar.Message = ex.Message;
            ErrorBar.IsOpen = true;
        }
    }

    private void RecentItem_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is string path)
            ProjectOpened?.Invoke(this, path);
    }
}
