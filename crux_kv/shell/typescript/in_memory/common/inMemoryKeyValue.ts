/**
 * A `KeyValueHandler` that keeps values in a `Map` for the life of the
 * handler.
 *
 * The app's handler holds an instance and delegates each operation to it:
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
 * Use it in tests, on a server or in a worker, or when the values must not
 * outlive the process. To keep values after a reload, write a
 * `KeyValueHandler` over a store that persists. `listKeys` answers with one
 * page, so the next cursor is always 0.
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
