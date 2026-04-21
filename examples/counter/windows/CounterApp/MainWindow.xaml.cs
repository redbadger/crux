using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Windows.Graphics;

namespace CounterApp;

public sealed partial class MainWindow : Window
{
    public CounterViewModel ViewModel { get; }

    public MainWindow(CounterViewModel viewModel)
    {
        ViewModel = viewModel;
        InitializeComponent();

        // Mica is Windows 11 only; older builds fall back to the default chrome.
        if (!MicaController.IsSupported())
        {
            SystemBackdrop = null;
        }

        const int width = 480;
        const int height = 320;
        AppWindow.Resize(new SizeInt32(width, height));

        var displayArea = DisplayArea.GetFromWindowId(AppWindow.Id, DisplayAreaFallback.Primary);
        AppWindow.Move(new PointInt32(
            displayArea.WorkArea.X + ((displayArea.WorkArea.Width - width) / 2),
            displayArea.WorkArea.Y + ((displayArea.WorkArea.Height - height) / 2)));

        Closed += (_, _) => ViewModel.Dispose();
    }
}
