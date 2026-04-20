using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Storage.Pickers;
using WinRT.Interop;
using ChickenScratch.Core.Compile;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;
using WinUIEx;

namespace ChickenScratch.App.Views;

public sealed partial class SettingsDialog : ContentDialog
{
    private AppSettings _settings = new();

    public SettingsDialog()
    {
        InitializeComponent();
        Loaded += OnLoaded;
    }

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        _settings = await Task.Run(SettingsService.GetSettings);
        PopulateFields();

        // Check pandoc version
        _ = Task.Run(() =>
        {
            try
            {
                var ver = PandocService.CheckPandoc();
                DispatcherQueue.TryEnqueue(() => PandocVersionText.Text = $"Detected: {ver}");
            }
            catch
            {
                DispatcherQueue.TryEnqueue(() => PandocVersionText.Text = "Not installed");
            }
        });
    }

    private void PopulateFields()
    {
        // General
        SelectByTag(ThemeBox, _settings.General.Theme);
        RecentLimitBox.Value = _settings.General.RecentProjectsLimit;
        PandocPathBox.Text = _settings.General.PandocPath ?? string.Empty;

        // Writing
        SelectByContent(FontFamilyBox, _settings.Writing.FontFamily);
        FontSizeBox.Value = _settings.Writing.FontSize;
        SelectByTag(ParagraphStyleBox, _settings.Writing.ParagraphStyle);
        AutoSaveBox.Value = _settings.Writing.AutoSaveSeconds;

        // Backup
        BackupDirBox.Text = _settings.Backup.BackupDirectory ?? string.Empty;
        AutoBackupCloseToggle.IsOn = _settings.Backup.AutoBackupOnClose;
        BackupIntervalBox.Value = _settings.Backup.AutoBackupMinutes;

        // AI
        AiEnabledToggle.IsOn = _settings.Ai.Enabled;
        AiFields.Visibility = _settings.Ai.Enabled ? Visibility.Visible : Visibility.Collapsed;
        SelectByTag(AiProviderBox, _settings.Ai.Provider);
        AiModelBox.Text = _settings.Ai.Model;
        AiEndpointBox.Text = _settings.Ai.Endpoint ?? string.Empty;
        AiApiKeyBox.Password = _settings.Ai.ApiKey ?? string.Empty;
        UpdateApiKeyVisibility();

        // Compile
        SelectByTag(CompileFormatBox, _settings.Compile.DefaultFormat);
        CompileFontBox.Text = _settings.Compile.Font;
        CompileFontSizeBox.Value = _settings.Compile.FontSize;
        SelectByTag(LineSpacingBox, _settings.Compile.LineSpacing.ToString());
        MarginsBox.Value = _settings.Compile.MarginInches;
    }

    private AppSettings CollectFields()
    {
        _settings.General.Theme = GetTag(ThemeBox, "dark");
        _settings.General.RecentProjectsLimit = (int)RecentLimitBox.Value;
        _settings.General.PandocPath = string.IsNullOrWhiteSpace(PandocPathBox.Text)
            ? null : PandocPathBox.Text.Trim();

        _settings.Writing.FontFamily = (FontFamilyBox.SelectedItem as ComboBoxItem)?.Content?.ToString()
            ?? "Segoe UI Variable";
        _settings.Writing.FontSize = (float)FontSizeBox.Value;
        _settings.Writing.ParagraphStyle = GetTag(ParagraphStyleBox, "block");
        _settings.Writing.AutoSaveSeconds = (int)AutoSaveBox.Value;

        _settings.Backup.BackupDirectory = string.IsNullOrWhiteSpace(BackupDirBox.Text)
            ? null : BackupDirBox.Text.Trim();
        _settings.Backup.AutoBackupOnClose = AutoBackupCloseToggle.IsOn;
        _settings.Backup.AutoBackupMinutes = (int)BackupIntervalBox.Value;

        _settings.Ai.Enabled = AiEnabledToggle.IsOn;
        _settings.Ai.Provider = GetTag(AiProviderBox, "ollama");
        _settings.Ai.Model = AiModelBox.Text.Trim();
        _settings.Ai.Endpoint = string.IsNullOrWhiteSpace(AiEndpointBox.Text)
            ? null : AiEndpointBox.Text.Trim();
        _settings.Ai.ApiKey = string.IsNullOrWhiteSpace(AiApiKeyBox.Password)
            ? null : AiApiKeyBox.Password;

        _settings.Compile.DefaultFormat = GetTag(CompileFormatBox, "docx");
        _settings.Compile.Font = CompileFontBox.Text.Trim();
        _settings.Compile.FontSize = (float)CompileFontSizeBox.Value;
        _settings.Compile.LineSpacing = float.TryParse(GetTag(LineSpacingBox, "2"), out var ls) ? ls : 2f;
        _settings.Compile.MarginInches = (float)MarginsBox.Value;

        return _settings;
    }

    private async void Save_Click(ContentDialog sender, ContentDialogButtonClickEventArgs args)
    {
        var settings = CollectFields();
        await Task.Run(() => SettingsService.SaveSettings(settings));
    }

    private async void BrowseBackup_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker();
        picker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        picker.FileTypeFilter.Add("*");
        WinRT.Interop.InitializeWithWindow.Initialize(picker,
            WindowNative.GetWindowHandle(App.MainWindow!));
        var folder = await picker.PickSingleFolderAsync();
        if (folder != null) BackupDirBox.Text = folder.Path;
    }

    private void AiEnabled_Toggled(object sender, RoutedEventArgs e)
        => AiFields.Visibility = AiEnabledToggle.IsOn ? Visibility.Visible : Visibility.Collapsed;

    private void AiProvider_Changed(object sender, SelectionChangedEventArgs e)
        => UpdateApiKeyVisibility();

    private void UpdateApiKeyVisibility()
    {
        var provider = GetTag(AiProviderBox, "ollama");
        ApiKeyField.Visibility = provider != "ollama" ? Visibility.Visible : Visibility.Collapsed;
    }

    // ── Helpers ───────────────────────────────────────

    private static void SelectByTag(ComboBox box, string tag)
    {
        foreach (var item in box.Items.OfType<ComboBoxItem>())
            if (item.Tag?.ToString() == tag) { box.SelectedItem = item; return; }
        if (box.Items.Count > 0) box.SelectedIndex = 0;
    }

    private static void SelectByContent(ComboBox box, string content)
    {
        foreach (var item in box.Items.OfType<ComboBoxItem>())
            if (item.Content?.ToString() == content) { box.SelectedItem = item; return; }
        if (box.Items.Count > 0) box.SelectedIndex = 0;
    }

    private static string GetTag(ComboBox box, string fallback)
        => (box.SelectedItem as ComboBoxItem)?.Tag?.ToString() ?? fallback;
}
