using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml;

namespace CounterApp;

public partial class App : Application
{
    private readonly ILoggerFactory loggerFactory = LoggerFactory.Create(b => b.AddDebug());
    private readonly ILogger<App> logger;

    // Held to prevent garbage collection of the main window for the life of the app.
    private Window? window;

    public App()
    {
        logger = loggerFactory.CreateLogger<App>();
        InitializeComponent();
        UnhandledException += OnUnhandledException;
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        var viewModel = new CounterViewModel(new Core(), loggerFactory.CreateLogger<CounterViewModel>());
        window = new MainWindow(viewModel);
        window.Activate();
    }

    private void OnUnhandledException(object sender, Microsoft.UI.Xaml.UnhandledExceptionEventArgs e)
    {
        LogUnhandled(logger, e.Exception);
    }

    [LoggerMessage(Level = LogLevel.Error, Message = "Unhandled exception")]
    private static partial void LogUnhandled(ILogger logger, Exception exception);
}
