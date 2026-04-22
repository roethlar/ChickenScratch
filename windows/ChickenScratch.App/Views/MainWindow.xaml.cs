using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using ChickenScratch.App.ViewModels;
using ChickenScratch.Core.IO;
using WinUIEx;

namespace ChickenScratch.App.Views;

public sealed partial class MainWindow : Window
{
    public AppViewModel ViewModel { get; } = new();

    public MainWindow()
    {
        InitializeComponent();

        if (Microsoft.UI.Composition.SystemBackdrops.MicaController.IsSupported())
            SystemBackdrop = new Microsoft.UI.Xaml.Media.MicaBackdrop();

        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);
        Title = "ChickenScratch";

        AppTitleBar.Loaded += (_, _) => UpdateTitleBarInset();
        AppWindow.Changed += (_, args) => { if (args.DidSizeChange) UpdateTitleBarInset(); };

        this.SetWindowSize(1280, 820);
        this.CenterOnScreen();

        var queue = DispatcherQueue;
        ViewModel.Editor.Initialize(queue);
        ViewModel.Inspector.Initialize(queue);

        var settings = SettingsService.GetSettings();
        ApplyTheme(settings.General.Theme);
    }

    public void ApplyTheme(string theme)
    {
        RootGrid.RequestedTheme = theme switch
        {
            "light" or "sepia" or "solarized-light" => ElementTheme.Light,
            _ => ElementTheme.Dark,
        };

        // Mica follows system theme, not RequestedTheme, so non-dark themes need explicit backgrounds.
        RootGrid.Background = theme switch
        {
            "light"          => new SolidColorBrush(Windows.UI.Color.FromArgb(255, 249, 249, 249)),
            "sepia"          => new SolidColorBrush(Windows.UI.Color.FromArgb(255, 251, 245, 224)),
            "solarized-light"=> new SolidColorBrush(Windows.UI.Color.FromArgb(255, 253, 246, 227)),
            "solarized-dark" => new SolidColorBrush(Windows.UI.Color.FromArgb(255,   0,  43,  54)),
            "dracula"        => new SolidColorBrush(Windows.UI.Color.FromArgb(255,  40,  42,  54)),
            _                => null, // dark: let Mica show
        };
    }

    private void UpdateTitleBarInset()
    {
        TitleBarRightButtons.Margin = new Thickness(0, 0, AppWindow.TitleBar.RightInset, 0);
    }

    private async void Welcome_ProjectOpened(object sender, string path)
        => await ViewModel.OpenProjectAsync(path);

    private async void Welcome_ProjectCreated(object sender, (string name, string path) args)
        => await ViewModel.CreateProjectAsync(args);

    private void Binder_DocumentSelected(object sender, string docId)
    {
        ViewModel.SelectDocument(docId);
        if (ViewModel.CurrentProject != null)
        {
            var projectPath = ViewModel.CurrentProject.Path;
            ViewModel.Editor.SetProjectPath(projectPath);
            ViewModel.Inspector.SetProjectPath(projectPath);
        }
    }

    private void ToggleBinder_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.ShowBinder = !ViewModel.ShowBinder;
        BinderColumn.Width = ViewModel.ShowBinder ? new GridLength(240) : GridLength.Auto;
        BinderPanel.Visibility = ViewModel.ShowBinder ? Visibility.Visible : Visibility.Collapsed;
    }

    private void ToggleInspector_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.ShowInspector = !ViewModel.ShowInspector;
        InspectorColumn.Width = ViewModel.ShowInspector ? new GridLength(260) : GridLength.Auto;
        InspectorPanel.Visibility = ViewModel.ShowInspector ? Visibility.Visible : Visibility.Collapsed;
    }

    private async void SaveRevision_Click(object sender, RoutedEventArgs e)
    {
        if (ViewModel.CurrentProject == null) return;

        var box = new TextBox { PlaceholderText = "Describe this revision\u2026", MinWidth = 320 };
        var dialog = new ContentDialog
        {
            Title = "Save Revision",
            Content = box,
            PrimaryButtonText = "Save",
            CloseButtonText = "Cancel",
            XamlRoot = Content.XamlRoot,
            DefaultButton = ContentDialogButton.Primary,
        };

        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
        {
            var msg = box.Text.Trim();
            await ViewModel.SaveRevisionAsync(string.IsNullOrEmpty(msg) ? "Manual save" : msg);
        }
    }

    private async void OpenSettings_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new SettingsDialog { XamlRoot = Content.XamlRoot };
        await dialog.ShowAsync();
        var settings = SettingsService.GetSettings();
        ApplyTheme(settings.General.Theme);
    }
}
