using Microsoft.UI.Xaml;
using ChickenScratch.App.Views;

namespace ChickenScratch.App;

public partial class App : Application
{
    private MainWindow? _window;

    public App() => InitializeComponent();

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();
        _window.Activate();
    }

    public static MainWindow? MainWindow => (Current as App)?._window;
}
