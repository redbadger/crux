import java.util.concurrent.ConcurrentHashMap

/**
 * A [KeyValueHandler] that keeps values in a map for the life of the handler.
 *
 * The app's handler holds an instance and delegates each operation to it:
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
 * Use it in tests, in previews, in a shell with no storage, or when the values
 * must not outlive the process. To keep values after the app stops, implement
 * [KeyValueHandler] over a store that persists. `listKeys` answers with one
 * page, so the next cursor is always 0.
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
