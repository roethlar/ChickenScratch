using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using ChickenScratch.App.ViewModels;

namespace ChickenScratch.App.Controls;

public sealed partial class InspectorControl : UserControl
{
    public static readonly DependencyProperty ViewModelProperty =
        DependencyProperty.Register(nameof(ViewModel), typeof(InspectorViewModel),
            typeof(InspectorControl), new PropertyMetadata(null));

    public InspectorViewModel ViewModel
    {
        get => (InspectorViewModel)GetValue(ViewModelProperty);
        set => SetValue(ViewModelProperty, value);
    }

    public InspectorControl() => InitializeComponent();
}
