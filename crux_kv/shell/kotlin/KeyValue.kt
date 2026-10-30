import java.util.concurrent.ConcurrentHashMap

/**
 * The shell side of `crux_kv`, as one method per operation.
 *
 * Each method's shape is the generated `EffectHandler`'s, so an app's handler
 * holds an instance and delegates in one line:
 *
 * ```kotlin
 * class MyHandler : EffectHandler {
 *     private val kv = InMemoryKeyValueHandler()
 *
 *     override suspend fun kvGet(operation: GetValue): ValueResult = kv.get(operation)
 *     override suspend fun kvSet(operation: SetValue): ValueResult = kv.set(operation)
 * }
 * ```
 *
 * One store ships: [InMemoryKeyValueHandler], which keeps values only in
 * memory. To keep values after the app stops, implement this interface over a
 * store that persists (`DataStore`, `SharedPreferences`, files or a database).
 * You can also implement it to replace a single method.
 *
 * ## What each operation answers with
 *
 * `get` answers with what is stored under the key, `Value.None` when nothing
 * is. `set` and `delete` answer with the value they replaced or removed, which
 * is also `Value.None` when there was none. Nothing here is an error unless
 * the store itself failed; a missing key is an answer.
 */
interface KeyValueHandler {
    /** Read the bytes stored under `operation.key`. */
    suspend fun get(operation: GetValue): ValueResult

    /** Write `operation.value`, answering with the value it replaced. */
    suspend fun set(operation: SetValue): ValueResult

    /** Remove `operation.key`, answering with the value it removed. */
    suspend fun delete(operation: DeleteValue): ValueResult

    /** Whether `operation.key` is in the store. */
    suspend fun exists(operation: KeyExists): ExistsResult

    /** The keys starting with `operation.prefix`, from `operation.cursor`. */
    suspend fun listKeys(operation: ListKeys): KeysResult
}

/**
 * A [KeyValueHandler] that keeps values in a map for the life of the handler.
 *
 * Use it in tests, in previews, in a shell with no storage, or when the values
 * must not outlive the process. `listKeys` answers with one page, so the next
 * cursor is always 0.
 */
class InMemoryKeyValueHandler : KeyValueHandler {
    private val values = ConcurrentHashMap<String, List<UByte>>()

    override suspend fun get(operation: GetValue): ValueResult = ValueResult.Ok(value(values[operation.key]))

    override suspend fun set(operation: SetValue): ValueResult =
        ValueResult.Ok(value(values.put(operation.key, operation.value.toList())))

    override suspend fun delete(operation: DeleteValue): ValueResult = ValueResult.Ok(value(values.remove(operation.key)))

    override suspend fun exists(operation: KeyExists): ExistsResult = ExistsResult.Ok(values.containsKey(operation.key))

    override suspend fun listKeys(operation: ListKeys): KeysResult {
        val keys = values.keys.filter { it.startsWith(operation.prefix) }.sorted()

        if (operation.cursor > keys.size.toULong()) {
            return KeysResult.Err(KeyValueError.CursorNotFound)
        }

        return KeysResult.Ok(KeyPage(keys.drop(operation.cursor.toInt()), 0uL))
    }

    /** A copy, so that the caller cannot change what is stored. */
    private fun value(bytes: List<UByte>?): Value = if (bytes == null) Value.None else Value.Bytes(bytes.toList())
}
