using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CounterApp.Shared;
using Microsoft.UI.Dispatching;

namespace CounterApp;

/// <summary>
/// Binds the generated <c>Core</c> to the window: the buttons send events to
/// it, and its <c>View</c> is mirrored into an observable property for
/// <c>x:Bind</c>.
/// </summary>
public sealed partial class CounterViewModel : ObservableObject
{
    private readonly Core core;
    private readonly DispatcherQueue dispatcherQueue = DispatcherQueue.GetForCurrentThread();

    [ObservableProperty]
    private ViewModel view;

    public CounterViewModel(Core core)
    {
        this.core = core;
        view = core.View;
        core.PropertyChanged += OnCorePropertyChanged;
    }

    [RelayCommand]
    private void Reset() => core.Update(Event.Reset);

    [RelayCommand]
    private void Increment() => core.Update(Event.Increment);

    [RelayCommand]
    private void Decrement() => core.Update(Event.Decrement);

    /// <summary>
    /// <c>Core</c> raises <c>PropertyChanged</c> on whichever thread processed
    /// the <c>Render</c>. For the counter that is always the UI thread, inside
    /// <c>Update</c>, but after an asynchronous effect it would be a
    /// thread-pool thread, and WinUI bindings must be updated on the UI
    /// thread, so hop there when we are not already on it.
    /// </summary>
    private void OnCorePropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName != nameof(Core.View))
        {
            return;
        }

        if (dispatcherQueue.HasThreadAccess)
        {
            View = core.View;
        }
        else
        {
            _ = dispatcherQueue.TryEnqueue(() => View = core.View);
        }
    }
}
