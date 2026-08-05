using System;
using System.Collections.Generic;
using CounterApp.Shared;

namespace CounterApp;

// Thin, disposable wrapper over BoltFFI's generated Rust core bindings. Holds
// no observable state (see CounterViewModel).
public sealed class Core : IDisposable
{
    private readonly CoreFfi ffi = new();

    public ViewModel View() =>
        ViewModel.BincodeDeserialize(ffi.View());

    public IReadOnlyList<Request> Update(Event @event) =>
        Requests.BincodeDeserialize(ffi.Update(EventBincode.BincodeSerialize(@event))).Value;

    public IReadOnlyList<Request> Resolve(uint id, byte[] data) =>
        Requests.BincodeDeserialize(ffi.Resolve(id, data)).Value;

    public void Dispose() => ffi.Dispose();
}
