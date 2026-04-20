using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using ChickenScratch.App.ViewModels;

namespace ChickenScratch.App.Converters;

// bool → Visibility (pass ConverterParameter="inverse" to flip)
public class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool flag = value is true;
        if (parameter is string p && p == "inverse") flag = !flag;
        return flag ? Visibility.Visible : Visibility.Collapsed;
    }
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => value is Visibility.Visible;
}

// !bool → Visibility
public class InverseBoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value is false ? Visibility.Visible : Visibility.Collapsed;
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => value is not Visibility.Visible;
}

// null → Visible (show Welcome when project is null)
public class NullToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value == null ? Visibility.Visible : Visibility.Collapsed;
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotImplementedException();
}

// not-null → Visible (show Editor when project is set)
public class NotNullToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value != null ? Visibility.Visible : Visibility.Collapsed;
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotImplementedException();
}

// bool (isDocument) → FontIcon glyph
public class NodeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value is true ? "\uE8A5" : "\uE8B7"; // document : folder
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotImplementedException();
}

// int wordCount → "1,234 words"
public class WordCountConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value is int wc ? $"{wc:N0} words" : string.Empty;
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotImplementedException();
}

// SaveStatus → label string
public class SaveStatusConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value switch
        {
            SaveStatus.Saved    => "Saved",
            SaveStatus.Modified => "Modified",
            SaveStatus.Saving   => "Saving…",
            _ => string.Empty,
        };
    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotImplementedException();
}
