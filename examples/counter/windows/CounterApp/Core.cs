using System;
using System.Collections.Generic;
using CounterApp.Shared;
using BoltCoreFFI = Shared.CoreFFI;

namespace CounterApp;

// Thin, disposable wrapper over BoltFFI's generated Rust core bindings. Holds
// no observable state (see CounterViewModel).
public sealed class Core : IDisposable
{
    private readonly BoltCoreFFI ffi = new();

    public ViewModel View() =>
        ViewModel.BincodeDeserialize(ffi.View());

    public IReadOnlyList<Request> Update(Event @event) =>
        Requests.BincodeDeserialize(ffi.UpdateBytes(EventBincode.BincodeSerialize(@event))).Value;

    public IReadOnlyList<Request> Resolve(uint id, byte[] data) =>
        Requests.BincodeDeserialize(ffi.ResolveBytes(id, data)).Value;

    public void Dispose() => ffi.Dispose();
}
