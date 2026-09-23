/**
 * The shell side of `crux_kv`, as one method per operation.
 *
 * Each method's shape is the generated `EffectHandler`'s, so an app's handler
 * holds an instance and delegates in one line:
 *
 * ```ts
 * const kv = createInMemoryKeyValueHandler();
 *
 * const handler: EffectHandler = {
 *   kvGet: (operation) => kv.get(operation),
 *   kvSet: (operation) => kv.set(operation),
 * };
 * ```
 *
 * One store ships: `createInMemoryKeyValueHandler`, which keeps values only in
 * memory. To keep values after a reload, write an object of this shape over a
 * store that persists (`localStorage`, IndexedDB or a server). You can also
 * write one to replace a single method.
 *
 * ## What each operation answers with
 *
 * `get` answers with what is stored under the key, `valueNone()` when nothing
 * is. `set` and `delete` answer with the value they replaced or removed, which
 * is also `valueNone()` when there was none. A missing key is an answer, not
 * an error. An operation answers with an error only when the store fails.
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

/**
 * A `KeyValueHandler` that keeps values in a `Map` for the life of the
 * handler.
 *
 * Use it in tests, on a server or in a worker, or when the values must not
 * outlive the process. `listKeys` answers with one page, so the next cursor is
 * always 0.
 */
export function createInMemoryKeyValueHandler(): KeyValueHandler {
  const values = new Map<string, number[]>();

  // A copy, so that the caller cannot change what is stored.
  const answer = (value: number[] | undefined): ValueResult =>
    valueResultOk(value === undefined ? valueNone() : valueBytes([...value]));

  return {
    async get(operation: GetValue): Promise<ValueResult> {
      return answer(values.get(operation.key));
    },

    async set(operation: SetValue): Promise<ValueResult> {
      const previous = values.get(operation.key);
      values.set(operation.key, Array.from(operation.value));
      return answer(previous);
    },

    async delete(operation: DeleteValue): Promise<ValueResult> {
      const previous = values.get(operation.key);
      values.delete(operation.key);
      return answer(previous);
    },

    async exists(operation: KeyExists): Promise<ExistsResult> {
      return existsResultOk(values.has(operation.key));
    },

    async listKeys(operation: ListKeys): Promise<KeysResult> {
      const keys = Array.from(values.keys()).filter((key) =>
        key.startsWith(operation.prefix),
      );
      keys.sort();

      const cursor = Number(operation.cursor);
      if (cursor > keys.length) return keysResultErr(keyValueErrorCursorNotFound());

      return keysResultOk(new KeyPage(keys.slice(cursor), BigInt(0)));
    },
  };
}
