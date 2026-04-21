using System;
using System.Collections.Generic;
using CounterApp.Shared;

namespace CounterApp;

// Thin, disposable wrapper over the Rust core's FFI surface. Owns the native
// handle via CounterCoreHandle; holds no observable state (see CounterViewModel).
public sealed class Core : IDisposable
{
    private readonly CounterCoreHandle handle = CounterCoreHandle.Create();

    public ViewModel View() =>
        ViewModel.BincodeDeserialize(CoreFfi.View(handle));

    public IReadOnlyList<Request> Update(Event @event) =>
        Requests.BincodeDeserialize(CoreFfi.Update(handle, EventBincode.BincodeSerialize(@event)));

    public IReadOnlyList<Request> Resolve(uint id, byte[] data) =>
        Requests.BincodeDeserialize(CoreFfi.Resolve(handle, id, data));

    public void Dispose() => handle.Dispose();
}
