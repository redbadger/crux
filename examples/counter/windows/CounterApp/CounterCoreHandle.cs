using System;
using Microsoft.Win32.SafeHandles;

namespace CounterApp;

// GC-tracked owner of a *CoreFFI produced by crux_counter_new. The runtime
// guarantees ReleaseHandle runs exactly once (on Dispose or finalization), and
// DangerousAddRef/Release protects against GC collecting the handle mid-call.
internal sealed class CounterCoreHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public CounterCoreHandle()
        : base(ownsHandle: true)
    {
    }

    public static CounterCoreHandle Create()
    {
        var safe = new CounterCoreHandle();
        safe.SetHandle(CoreFfi.NativeNew());
        if (safe.IsInvalid)
        {
            safe.Dispose();
            throw new InvalidOperationException("crux_counter_new returned an invalid handle.");
        }

        return safe;
    }

    protected override bool ReleaseHandle()
    {
        CoreFfi.NativeFree(handle);
        return true;
    }
}
