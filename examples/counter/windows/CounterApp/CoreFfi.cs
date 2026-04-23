using System;
using System.Runtime.InteropServices;

namespace CounterApp;

internal static partial class CoreFfi
{
    private const string Dll = "shared";

    [StructLayout(LayoutKind.Sequential)]
    private struct ByteBuf
    {
        public nint Ptr;
        public nuint Len;
        public nuint Cap;
    }

    [LibraryImport(Dll, EntryPoint = "crux_counter_new")]
    internal static partial nint NativeNew();

    [LibraryImport(Dll, EntryPoint = "crux_counter_free")]
    internal static partial void NativeFree(nint core);

    [LibraryImport(Dll, EntryPoint = "crux_counter_update")]
    private static partial void NativeUpdate(nint core, ReadOnlySpan<byte> data, nuint len, out ByteBuf outBuf);

    [LibraryImport(Dll, EntryPoint = "crux_counter_resolve")]
    private static partial void NativeResolve(nint core, uint id, ReadOnlySpan<byte> data, nuint len, out ByteBuf outBuf);

    [LibraryImport(Dll, EntryPoint = "crux_counter_view")]
    private static partial void NativeView(nint core, out ByteBuf outBuf);

    [LibraryImport(Dll, EntryPoint = "crux_counter_free_buf")]
    private static partial void NativeFreeBuf(ByteBuf buf);

    public static byte[] Update(CounterCoreHandle core, ReadOnlySpan<byte> @event)
    {
        var added = false;
        try
        {
            core.DangerousAddRef(ref added);
            NativeUpdate(core.DangerousGetHandle(), @event, (nuint)@event.Length, out var buf);
            return ReadAndFree(buf);
        }
        finally
        {
            if (added)
            {
                core.DangerousRelease();
            }
        }
    }

    public static byte[] Resolve(CounterCoreHandle core, uint id, ReadOnlySpan<byte> data)
    {
        var added = false;
        try
        {
            core.DangerousAddRef(ref added);
            NativeResolve(core.DangerousGetHandle(), id, data, (nuint)data.Length, out var buf);
            return ReadAndFree(buf);
        }
        finally
        {
            if (added)
            {
                core.DangerousRelease();
            }
        }
    }

    public static byte[] View(CounterCoreHandle core)
    {
        var added = false;
        try
        {
            core.DangerousAddRef(ref added);
            NativeView(core.DangerousGetHandle(), out var buf);
            return ReadAndFree(buf);
        }
        finally
        {
            if (added)
            {
                core.DangerousRelease();
            }
        }
    }

    private static byte[] ReadAndFree(ByteBuf buf)
    {
        try
        {
            var result = new byte[checked((int)buf.Len)];
            if (buf.Len > 0)
            {
                Marshal.Copy(buf.Ptr, result, 0, result.Length);
            }

            return result;
        }
        finally
        {
            NativeFreeBuf(buf);
        }
    }
}
