using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Threading.Tasks;

/// <summary>
/// An <see cref="IKeyValueHandler"/> that keeps values in a dictionary for the
/// life of the handler.
/// </summary>
/// <remarks>
/// <para>
/// The app's handler holds an instance and delegates each operation to it:
/// </para>
/// <code>
/// public sealed class MyHandler : IEffectHandler
/// {
///     private readonly IKeyValueHandler _kv = new InMemoryKeyValueHandler();
///
///     public Task&lt;ValueResult&gt; KvGet(GetValue operation) => _kv.Get(operation);
///     public Task&lt;ValueResult&gt; KvSet(SetValue operation) => _kv.Set(operation);
/// }
/// </code>
/// <para>
/// Use it in tests, in previews, in a shell with no storage, or when the values
/// must not outlive the process. To keep values after the app stops, implement
/// <see cref="IKeyValueHandler"/> over a store that persists. <c>ListKeys</c>
/// answers with one page, so the next cursor is always 0.
/// </para>
/// </remarks>
public sealed class InMemoryKeyValueHandler : IKeyValueHandler
{
    private readonly Dictionary<string, byte[]> _values = new();
    private readonly object _lock = new();

    public Task<ValueResult> Get(GetValue operation)
    {
        lock (_lock)
        {
            _values.TryGetValue(operation.Key, out var value);
            return Answer(value);
        }
    }

    public Task<ValueResult> Set(SetValue operation)
    {
        var bytes = new byte[operation.Value.Count];
        operation.Value.CopyTo(bytes, 0);
        lock (_lock)
        {
            _values.TryGetValue(operation.Key, out var previous);
            _values[operation.Key] = bytes;
            return Answer(previous);
        }
    }

    public Task<ValueResult> Delete(DeleteValue operation)
    {
        lock (_lock)
        {
            _values.Remove(operation.Key, out var previous);
            return Answer(previous);
        }
    }

    public Task<ExistsResult> Exists(KeyExists operation)
    {
        lock (_lock)
        {
            return Task.FromResult<ExistsResult>(new ExistsResult.Ok(_values.ContainsKey(operation.Key)));
        }
    }

    public Task<KeysResult> ListKeys(ListKeys operation)
    {
        var keys = new List<string>();
        lock (_lock)
        {
            foreach (var key in _values.Keys)
            {
                if (key.StartsWith(operation.Prefix, StringComparison.Ordinal))
                {
                    keys.Add(key);
                }
            }
        }
        keys.Sort(StringComparer.Ordinal);

        if (operation.Cursor > (ulong)keys.Count)
        {
            return Task.FromResult<KeysResult>(new KeysResult.Err(new KeyValueError.CursorNotFound()));
        }

        var page = keys.GetRange((int)operation.Cursor, keys.Count - (int)operation.Cursor);
        return Task.FromResult<KeysResult>(new KeysResult.Ok(new KeyPage
        {
            Keys = new ObservableCollection<string>(page),
            NextCursor = 0,
        }));
    }

    /// <summary>A copy, so that the caller cannot change what is stored.</summary>
    private static Task<ValueResult> Answer(byte[]? value) =>
        Task.FromResult<ValueResult>(value is null
            ? new ValueResult.Ok(new Value.None())
            : new ValueResult.Ok(new Value.Bytes(new ObservableCollection<byte>(value))));
}
