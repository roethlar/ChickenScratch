using System.Text.Json;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.Web.WebView2.Core;
using ChickenScratch.App.ViewModels;

namespace ChickenScratch.App.Controls;

public sealed partial class EditorControl : UserControl
{
    public static readonly DependencyProperty ViewModelProperty =
        DependencyProperty.Register(nameof(ViewModel), typeof(EditorViewModel),
            typeof(EditorControl), new PropertyMetadata(null, OnViewModelChanged));

    public EditorViewModel? ViewModel
    {
        get => (EditorViewModel?)GetValue(ViewModelProperty);
        set => SetValue(ViewModelProperty, value);
    }

    private Microsoft.UI.Xaml.Controls.WebView2? _webView;
    private bool _webViewReady;
    private string? _pendingContent;

    public EditorControl()
    {
        InitializeComponent();
        Loaded += OnLoadedAsync;
    }

    private async void OnLoadedAsync(object sender, RoutedEventArgs e)
    {
        _webView = new Microsoft.UI.Xaml.Controls.WebView2();
        WebViewHost.Content = _webView;

        await _webView.EnsureCoreWebView2Async();

        _webView.CoreWebView2.WebMessageReceived += OnWebMessageReceived;
        _webView.CoreWebView2.Settings.IsWebMessageEnabled = true;
        _webView.CoreWebView2.Settings.AreDevToolsEnabled = false;
        _webView.CoreWebView2.Settings.IsStatusBarEnabled = false;

        // Map local folder so editor.html can use relative imports if needed
        _webView.CoreWebView2.SetVirtualHostNameToFolderMapping(
            "editor.local",
            Path.Combine(AppContext.BaseDirectory, "Editor"),
            CoreWebView2HostResourceAccessKind.Allow);

        // Wait for NavigationCompleted before marking ready
        _webView.CoreWebView2.NavigationCompleted += OnNavigationCompleted;

        _webView.Source = new Uri("https://editor.local/editor.html");
    }

    private async void OnNavigationCompleted(CoreWebView2 sender, CoreWebView2NavigationCompletedEventArgs e)
    {
        _webViewReady = true;

        if (_pendingContent != null)
        {
            await SendToEditorAsync("setContent", [_pendingContent]);
            _pendingContent = null;
        }
    }

    private void OnWebMessageReceived(CoreWebView2 sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        try
        {
            var json = e.TryGetWebMessageAsString();
            var msg = JsonSerializer.Deserialize<JsonElement>(json);
            var type = msg.GetProperty("type").GetString();

            DispatcherQueue.TryEnqueue(() =>
            {
                switch (type)
                {
                    case "contentChanged" when ViewModel != null:
                        var html = msg.GetProperty("content").GetString() ?? string.Empty;
                        ViewModel.OnContentChanged(html, 0);
                        break;

                    case "wordCount" when ViewModel != null:
                        ViewModel.WordCount = msg.GetProperty("count").GetInt32();
                        break;

                    case "selectionFormat" when ViewModel != null:
                        ViewModel.OnSelectionFormatChanged(
                            msg.GetProperty("bold").GetBoolean(),
                            msg.GetProperty("italic").GetBoolean(),
                            msg.GetProperty("underline").GetBoolean());
                        break;
                }
            });
        }
        catch { /* ignore malformed messages */ }
    }

    private static void OnViewModelChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not EditorControl ctrl) return;

        if (e.OldValue is EditorViewModel old)
            old.PropertyChanged -= ctrl.OnViewModelPropertyChanged;

        if (e.NewValue is EditorViewModel vm)
        {
            vm.PropertyChanged += ctrl.OnViewModelPropertyChanged;
            _ = ctrl.SetEditorContent(vm.HtmlContent);
        }
    }

    private void OnViewModelPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(EditorViewModel.HtmlContent) && ViewModel != null)
            _ = SetEditorContent(ViewModel.HtmlContent);
    }

    private async Task SetEditorContent(string html)
    {
        if (!_webViewReady) { _pendingContent = html; return; }
        await SendToEditorAsync("setContent", [html]);
    }

    // Uses PostWebMessageAsString — the only way the JS addEventListener('message') bridge fires
    private async Task SendToEditorAsync(string command, object[]? args = null)
    {
        if (_webView?.CoreWebView2 == null || !_webViewReady) return;
        var payload = JsonSerializer.Serialize(new { command, args = args ?? Array.Empty<object>() });
        _webView.CoreWebView2.PostWebMessageAsString(payload);
        await Task.CompletedTask;
    }

    // Toolbar handlers
    private void Bold_Click(object s, RoutedEventArgs e)      => _ = SendToEditorAsync("bold");
    private void Italic_Click(object s, RoutedEventArgs e)    => _ = SendToEditorAsync("italic");
    private void Underline_Click(object s, RoutedEventArgs e) => _ = SendToEditorAsync("underline");
    private void H1_Click(object s, RoutedEventArgs e)        => _ = SendToEditorAsync("heading", [1]);
    private void H2_Click(object s, RoutedEventArgs e)        => _ = SendToEditorAsync("heading", [2]);
    private void H3_Click(object s, RoutedEventArgs e)        => _ = SendToEditorAsync("heading", [3]);
    private void Undo_Click(object s, RoutedEventArgs e)      => _ = SendToEditorAsync("undo");
    private void Redo_Click(object s, RoutedEventArgs e)      => _ = SendToEditorAsync("redo");

    private void Find_Click(object sender, RoutedEventArgs e)
    {
        FindBar.Visibility = FindBar.Visibility == Visibility.Visible
            ? Visibility.Collapsed : Visibility.Visible;
        if (FindBar.Visibility == Visibility.Visible)
            FindBox.Focus(FocusState.Programmatic);
    }

    private void FindBox_TextChanged(object sender, TextChangedEventArgs e)
        => _ = SendToEditorAsync("find", [FindBox.Text]);

    private void FindBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == Windows.System.VirtualKey.Escape) CloseFind();
    }

    private void CloseFind_Click(object sender, RoutedEventArgs e) => CloseFind();

    private void CloseFind()
    {
        FindBar.Visibility = Visibility.Collapsed;
        FindBox.Text = string.Empty;
    }

    private void Replace_Click(object sender, RoutedEventArgs e)
    {
        if (!string.IsNullOrEmpty(FindBox.Text))
            _ = SendToEditorAsync("replaceAll", [FindBox.Text, ReplaceBox.Text]);
    }
}
