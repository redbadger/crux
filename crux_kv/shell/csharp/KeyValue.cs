using System.Threading.Tasks;

/// <summary>
/// The shell side of <c>crux_kv</c>, as one method per operation.
/// </summary>
/// <remarks>
/// <para>
/// The shape of each method is the shape of the matching method of the
/// generated <c>IEffectHandler</c>. Thus an app's handler can hold an
/// <c>IKeyValueHandler</c> and delegate each operation to it in one line.
/// </para>
/// <para>
/// Implement this interface over a store, such as files or a database.
/// </para>
/// <para>
/// <c>Get</c> answers with the value that is stored under the key, or with
/// <c>Value.None</c> if the key has no value. <c>Set</c> and <c>Delete</c>
/// answer with the value that they replaced or removed, which is also
/// <c>Value.None</c> if there was none. A missing key is an answer, not an
/// error. An operation answers with an error only when the store fails.
/// </para>
/// </remarks>
public interface IKeyValueHandler
{
    /// <summary>Read the bytes stored under <c>operation.Key</c>.</summary>
    Task<ValueResult> Get(GetValue operation);

    /// <summary>Write <c>operation.Value</c>, answering with the value it replaced.</summary>
    Task<ValueResult> Set(SetValue operation);

    /// <summary>Remove <c>operation.Key</c>, answering with the value it removed.</summary>
    Task<ValueResult> Delete(DeleteValue operation);

    /// <summary>Whether <c>operation.Key</c> is in the store.</summary>
    Task<ExistsResult> Exists(KeyExists operation);

    /// <summary>The keys starting with <c>operation.Prefix</c>, from <c>operation.Cursor</c>.</summary>
    Task<KeysResult> ListKeys(ListKeys operation);
}
