/// The shell side of `crux_kv`, as one method per operation.
///
/// The shape of each method is the shape of the matching method of the
/// generated `EffectHandler`. Thus an app's handler can hold a
/// `KeyValueHandler` and delegate each operation to it in one line.
///
/// Conform a type to this protocol over a store (`UserDefaults`, a file, the
/// keychain or a database).
///
/// ## What each operation answers with
///
/// `get` answers with the value that is stored under the key, or with
/// `Value.none` if the key has no value. `set` and `delete` answer with the
/// value that they replaced or removed, which is also `Value.none` if there was
/// none. A missing key is an answer, not an error. An operation answers with an
/// error only when the store fails.
public protocol KeyValueHandler: Sendable {
    /// Read the bytes stored under `operation.key`.
    func get(_ operation: GetValue) async -> ValueResult
    /// Write `operation.value`, answering with the value it replaced.
    func set(_ operation: SetValue) async -> ValueResult
    /// Remove `operation.key`, answering with the value it removed.
    func delete(_ operation: DeleteValue) async -> ValueResult
    /// Whether `operation.key` is in the store.
    func exists(_ operation: KeyExists) async -> ExistsResult
    /// The keys starting with `operation.prefix`, from `operation.cursor`.
    func listKeys(_ operation: ListKeys) async -> KeysResult
}
