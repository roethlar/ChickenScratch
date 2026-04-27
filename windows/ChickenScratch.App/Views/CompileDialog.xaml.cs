using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.Storage.Pickers;
using Windows.UI;
using WinRT.Interop;
using ChickenScratch.Core.Compile;
using ChickenScratch.Core.Models;

namespace ChickenScratch.App.Views;

public sealed partial class CompileDialog : ContentDialog
{
    private readonly string _projectPath;

    public CompileDialog(string projectPath, ProjectMetadata metadata)
    {
        InitializeComponent();
        _projectPath = projectPath;
        TitleBox.Text  = metadata.Title  ?? string.Empty;
        AuthorBox.Text = metadata.Author ?? string.Empty;
    }

    private async void BrowseOutput_Click(object sender, RoutedEventArgs e)
    {
        var ext = SelectedFormat();
        var picker = new FileSavePicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            SuggestedFileName      = string.IsNullOrWhiteSpace(TitleBox.Text) ? "manuscript" : TitleBox.Text.Trim(),
        };
        picker.FileTypeChoices.Add(FormatLabel(ext), new List<string> { $".{ext}" });

        var hwnd = WindowNative.GetWindowHandle(App.MainWindow!);
        InitializeWithWindow.Initialize(picker, hwnd);

        var file = await picker.PickSaveFileAsync();
        if (file != null) OutputPathBox.Text = file.Path;
    }

    private async void Compile_Click(ContentDialog sender, ContentDialogButtonClickEventArgs args)
    {
        args.Cancel = true;

        var outputPath = OutputPathBox.Text.Trim();
        if (string.IsNullOrEmpty(outputPath))
        {
            ShowStatus("Please choose an output location.", isError: true);
            return;
        }

        IsPrimaryButtonEnabled = false;
        CompileProgress.Visibility = Visibility.Visible;
        ShowStatus("Compiling…", isError: false);

        var format    = SelectedFormat();
        var title     = NullIfEmpty(TitleBox.Text);
        var author    = NullIfEmpty(AuthorBox.Text);
        var separator = SeparatorBox.Text;
        var titlePage = IncludeTitlePageBox.IsChecked == true;
        var opts      = new CompileOptions { SectionSeparator = separator, IncludeTitlePage = titlePage };
        var path      = _projectPath;

        try
        {
            await Task.Run(() => PandocService.Compile(path, outputPath, format, title, author, opts));

            CompileProgress.Visibility = Visibility.Collapsed;
            ShowStatus($"Saved: {System.IO.Path.GetFileName(outputPath)}", isError: false, isSuccess: true);
            PrimaryButtonText = "Close";
            CloseButtonText   = string.Empty;
            IsPrimaryButtonEnabled = true;
        }
        catch (Exception ex)
        {
            CompileProgress.Visibility = Visibility.Collapsed;
            ShowStatus($"Error: {ex.Message}", isError: true);
            IsPrimaryButtonEnabled = true;
        }
    }

    private void ShowStatus(string message, bool isError, bool isSuccess = false)
    {
        StatusArea.Visibility = Visibility.Visible;
        StatusText.Text = message;
        StatusText.Foreground = isError   ? new SolidColorBrush(Color.FromArgb(255, 220, 80, 80))
                              : isSuccess ? new SolidColorBrush(Color.FromArgb(255, 100, 200, 100))
                              : null;
    }

    private string SelectedFormat()
        => (FormatBox.SelectedItem as ComboBoxItem)?.Tag as string ?? "docx";

    private string FormatLabel(string ext) => ext switch
    {
        "epub" => "EPUB",
        "html" => "HTML",
        "odt"  => "OpenDocument",
        _      => "Word Document",
    };

    private static string? NullIfEmpty(string? s)
        => string.IsNullOrWhiteSpace(s) ? null : s!.Trim();
}
