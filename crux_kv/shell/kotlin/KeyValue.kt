/**
 * The shell side of `crux_kv`, as one method per operation.
 *
 * The shape of each method is the shape of the matching method of the
 * generated `EffectHandler`. Thus an app's handler can hold a
 * `KeyValueHandler` and delegate each operation to it in one line.
 *
 * Implement this interface over a store (`DataStore`, `SharedPreferences`,
 * files or a database).
 *
 * ## What each operation answers with
 *
 * `get` answers with the value that is stored under the key, or with
 * `Value.None` if the key has no value. `set` and `delete` answer with the
 * value that they replaced or removed, which is also `Value.None` if there was
 * none. A missing key is an answer, not an error. An operation answers with an
 * error only when the store fails.
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
