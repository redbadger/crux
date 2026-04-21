using System;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CounterApp.Shared;
using Microsoft.Extensions.Logging;

namespace CounterApp;

public sealed partial class CounterViewModel : ObservableObject, IDisposable
{
    private readonly Core core;
    private readonly ILogger<CounterViewModel> logger;

    [ObservableProperty]
    private ViewModel view;

    public CounterViewModel(Core core, ILogger<CounterViewModel> logger)
    {
        this.core = core;
        this.logger = logger;
        view = core.View();
    }

    [RelayCommand]
    private void Reset() => Dispatch(Event.Reset);

    [RelayCommand]
    private void Increment() => Dispatch(Event.Increment);

    [RelayCommand]
    private void Decrement() => Dispatch(Event.Decrement);

    private void Dispatch(Event @event)
    {
        foreach (var request in core.Update(@event))
        {
            ProcessEffect(request);
        }
    }

    private void ProcessEffect(Request request)
    {
        switch (request.Effect)
        {
            case Effect.Render:
                View = core.View();
                break;
            default:
                // Other effects (HTTP, SSE) are handled by the other counter shells;
                // the Windows shell only wires up Render for this demo.
                LogUnhandledEffect(logger, request.Effect);
                break;
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Unhandled effect: {Effect}")]
    private static partial void LogUnhandledEffect(ILogger logger, Effect effect);

    public void Dispose() => core.Dispose();
}
