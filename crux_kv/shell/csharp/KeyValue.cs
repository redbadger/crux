using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Threading.Tasks;

/// <summary>
/// The shell side of <c>crux_kv</c>, as one method per operation.
/// </summary>
/// <remarks>
/// <para>
/// Each method's shape is the generated <c>IEffectHandler</c>'s, so an app's
/// handler holds an instance and delegates in one line:
/// </para>
/// <code>
/// public sealed class MyHandler : IEffectHandler
/// {
///     private readonly IKeyValueHandler _kv = new InMemoryKeyValueHandler();
///
///     public Task&lt;ValueResult&gt; KvGet(Get operation) => _kv.Get(operation);
///     public Task&lt;ValueResult&gt; KvSet(Set operation) => _kv.Set(operation);
/// }
/// </code>
/// <para>
/// One store ships: <see cref="InMemoryKeyValueHandler"/>, which keeps values
/// only in memory. To keep values after the app stops, implement this interface
/// over a store that persists, such as files or a database. You can also
/// implement it to replace a single method.
/// </para>
/// <para>
/// <c>Get</c> answers with what is stored under the key, <c>Value.None</c> when
/// nothing is. <c>Set</c> and <c>Delete</c> answer with the value they replaced
/// or removed, which is also <c>Value.None</c> when there was none. Nothing
/// here is an error unless the store itself failed; a missing key is an answer.
/// </para>
/// </remarks>
public interface IKeyValueHandler
{
    /// <summary>Read the bytes stored under <c>operation.Key</c>.</summary>
    Task<ValueResult> Get(Get operation);

    /// <summary>Write <c>operation.Value</c>, answering with the value it replaced.</summary>
    Task<ValueResult> Set(Set operation);

    /// <summary>Remove <c>operation.Key</c>, answering with the value it removed.</summary>
    Task<ValueResult> Delete(Delete operation);

    /// <summary>Whether <c>operation.Key</c> is in the store.</summary>
    Task<BoolResult> Exists(Exists operation);

    /// <summary>The keys starting with <c>operation.Prefix</c>, from <c>operation.Cursor</c>.</summary>
    Task<KeysResult> ListKeys(ListKeys operation);
}

/// <summary>
/// An <see cref="IKeyValueHandler"/> that keeps values in a dictionary for the
/// life of the handler.
/// </summary>
/// <remarks>
/// <para>
/// Use it in tests, in previews, in a shell with no storage, or when the values
/// must not outlive the process. <c>ListKeys</c> answers with one page, so the
/// next cursor is always 0.
/// </para>
/// </remarks>
public sealed class InMemoryKeyValueHandler : IKeyValueHandler
{
    private readonly Dictionary<string, byte[]> _values = new();
    private readonly object _lock = new();

    public Task<ValueResult> Get(Get operation)
    {
        lock (_lock)
        {
            _values.TryGetValue(operation.Key, out var value);
            return Answer(value);
        }
    }

    public Task<ValueResult> Set(Set operation)
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

    public Task<ValueResult> Delete(Delete operation)
    {
        lock (_lock)
        {
            _values.Remove(operation.Key, out var previous);
            return Answer(previous);
        }
    }

    public Task<BoolResult> Exists(Exists operation)
    {
        lock (_lock)
        {
            return Task.FromResult<BoolResult>(new BoolResult.Ok(_values.ContainsKey(operation.Key)));
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
