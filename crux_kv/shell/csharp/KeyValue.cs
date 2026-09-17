using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
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
///     private readonly IKeyValueHandler _kv = new FileKeyValueHandler(directory);
///
///     public Task&lt;ValueResult&gt; KvGet(Get operation) => _kv.Get(operation);
///     public Task&lt;ValueResult&gt; KvSet(Set operation) => _kv.Set(operation);
/// }
/// </code>
/// <para>
/// Implement it yourself to put the store somewhere else, or to replace a
/// single method and leave the rest.
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
/// An <see cref="IKeyValueHandler"/> that keeps one file per key in a directory.
/// </summary>
/// <remarks>
/// <para>
/// The BCL has no key-value store of its own, and files are what every runtime
/// has: no database and no dependency, and a store an app can point at its
/// application-data folder, a temporary directory in a test, or anywhere else
/// it can write.
/// </para>
/// <para>
/// A key is percent-encoded to make its file name, so a key with a
/// <c>/</c> in it is one file and not a directory, and <c>ListKeys</c> can read
/// the keys back out of the directory listing.
/// </para>
/// <para>
/// A write is atomic and flushed to the device, so a value survives a crash
/// whole or not at all. Beyond that this is a plain file store: every read goes
/// to the filesystem, there is no cache, nothing spans more than one key, and
/// nothing observes a change. A shell that wants those conforms its own type to
/// <c>IKeyValueHandler</c> and provides that instead.
/// </para>
/// </remarks>
public sealed class FileKeyValueHandler : IKeyValueHandler
{
    private readonly string _directory;

    /// <summary>Keeps the files in <paramref name="directory"/>, created on first use.</summary>
    public FileKeyValueHandler(string directory)
    {
        _directory = directory;
    }

    public Task<ValueResult> Get(Get operation) =>
        Task.FromResult(Io(() => new ValueResult.Ok(Read(Path(operation.Key)))));

    public Task<ValueResult> Set(Set operation) =>
        Task.FromResult(Io(() =>
        {
            var path = Path(operation.Key);
            var previous = Read(path);
            Directory.CreateDirectory(_directory);
            var bytes = new byte[operation.Value.Count];
            operation.Value.CopyTo(bytes, 0);
            Write(path, bytes);
            return new ValueResult.Ok(previous);
        }));

    public Task<ValueResult> Delete(Delete operation) =>
        Task.FromResult(Io(() =>
        {
            var path = Path(operation.Key);
            var previous = Read(path);
            // `File.Delete` ignores a file that is not there, but throws
            // `DirectoryNotFoundException` when the directory holding it is
            // not there either, which a store nothing has been written to yet
            // is. Removing a key that was never set is an answer, not an
            // error, so the delete only happens when there is something to
            // delete.
            if (previous is not Value.None)
            {
                File.Delete(path);
            }
            return new ValueResult.Ok(previous);
        }));

    public Task<BoolResult> Exists(Exists operation)
    {
        try
        {
            return Task.FromResult<BoolResult>(new BoolResult.Ok(File.Exists(Path(operation.Key))));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return Task.FromResult<BoolResult>(new BoolResult.Err(new KeyValueError.Io(e.Message)));
        }
    }

    /// <summary>
    /// One page holds every remaining key, so the answer's cursor is always 0:
    /// the directory is listed in one go anyway, and paging it would only
    /// invite a caller to hold a cursor across a write.
    /// </summary>
    public Task<KeysResult> ListKeys(ListKeys operation)
    {
        List<string> keys;
        try
        {
            keys = new List<string>();
            if (Directory.Exists(_directory))
            {
                foreach (var file in Directory.EnumerateFiles(_directory))
                {
                    var key = Uri.UnescapeDataString(System.IO.Path.GetFileName(file));
                    if (key.StartsWith(operation.Prefix, StringComparison.Ordinal))
                    {
                        keys.Add(key);
                    }
                }
            }
            keys.Sort(StringComparer.Ordinal);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return Task.FromResult<KeysResult>(new KeysResult.Err(new KeyValueError.Io(e.Message)));
        }

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

    private static Value Read(string path)
    {
        if (!File.Exists(path))
        {
            return new Value.None();
        }
        return new Value.Bytes(new ObservableCollection<byte>(File.ReadAllBytes(path)));
    }

    private string Path(string key) =>
        System.IO.Path.Combine(_directory, Uri.EscapeDataString(key));

    /// <summary>
    /// Writes <paramref name="bytes"/> to <paramref name="target"/> so that a
    /// reader sees either the whole of the new value or the whole of the old
    /// one.
    /// </summary>
    /// <remarks>
    /// The bytes go to a temporary file in a <c>tmp</c> directory inside the
    /// store, are flushed to the device, and are then moved over the target: a
    /// move within one volume is a rename, so a crash cannot leave a
    /// half-written value behind. The temporaries are in a directory of their
    /// own so that one left behind by a killed process is never mistaken for a
    /// key, since <c>ListKeys</c> reads the store's files and a directory is
    /// not one of them.
    /// </remarks>
    private void Write(string target, byte[] bytes)
    {
        var temporaries = System.IO.Path.Combine(_directory, "tmp");
        Directory.CreateDirectory(temporaries);
        var temporary = System.IO.Path.Combine(temporaries, System.IO.Path.GetRandomFileName());
        try
        {
            using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write))
            {
                stream.Write(bytes, 0, bytes.Length);
                stream.Flush(flushToDisk: true);
            }
            File.Move(temporary, target, overwrite: true);
        }
        finally
        {
            if (File.Exists(temporary))
            {
                File.Delete(temporary);
            }
        }
    }

    private static ValueResult Io(Func<ValueResult> block)
    {
        try
        {
            return block();
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return new ValueResult.Err(new KeyValueError.Io(e.Message));
        }
    }
}
