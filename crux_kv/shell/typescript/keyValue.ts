/**
 * The shell side of `crux_kv`, as one method per operation.
 *
 * The shape of each method is the shape of the matching method of the
 * generated `EffectHandler`. Thus an app's handler can hold a
 * `KeyValueHandler` and delegate each operation to it in one line.
 *
 * Write an object of this shape over a store (`localStorage`, IndexedDB or a
 * server).
 *
 * ## What each operation answers with
 *
 * `get` answers with the value that is stored under the key, or with
 * `valueNone()` if the key has no value. `set` and `delete` answer with the
 * value that they replaced or removed, which is also `valueNone()` if there was
 * none. A missing key is an answer, not an error. An operation answers with an
 * error only when the store fails.
 */
export interface KeyValueHandler {
  /** Read the bytes stored under `operation.key`. */
  get(operation: GetValue): Promise<ValueResult>;
  /** Write `operation.value`, answering with the value it replaced. */
  set(operation: SetValue): Promise<ValueResult>;
  /** Remove `operation.key`, answering with the value it removed. */
  delete(operation: DeleteValue): Promise<ValueResult>;
  /** Whether `operation.key` is in the store. */
  exists(operation: KeyExists): Promise<ExistsResult>;
  /** The keys starting with `operation.prefix`, from `operation.cursor`. */
  listKeys(operation: ListKeys): Promise<KeysResult>;
}
