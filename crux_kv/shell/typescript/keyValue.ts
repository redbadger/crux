/// The shell side of `crux_kv`, as one method per operation.
///
/// Each method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```ts
/// const kv = createLocalStorageKeyValueHandler();
///
/// const handler: EffectHandler = {
///   kvGet: (operation) => kv.get(operation),
///   kvSet: (operation) => kv.set(operation),
/// };
/// ```
///
/// Write an object of your own shape to put the store somewhere else —
/// IndexedDB, a server, `sessionStorage` — or to replace a single method and
/// leave the rest.
///
/// ## What each operation answers with
///
/// `get` answers with what is stored under the key, `valueNone()` when nothing
/// is. `set` and `delete` answer with the value they replaced or removed, which
/// is also `valueNone()` when there was none. Nothing here is an error unless
/// the store itself failed; a missing key is an answer.
export interface KeyValueHandler {
  /// Read the bytes stored under `operation.key`.
  get(operation: Get): Promise<ValueResult>;
  /// Write `operation.value`, answering with the value it replaced.
  set(operation: Set): Promise<ValueResult>;
  /// Remove `operation.key`, answering with the value it removed.
  delete(operation: Delete): Promise<ValueResult>;
  /// Whether `operation.key` is in the store.
  exists(operation: Exists): Promise<BoolResult>;
  /// The keys starting with `operation.prefix`, from `operation.cursor`.
  listKeys(operation: ListKeys): Promise<KeysResult>;
}

/// A `KeyValueHandler` over `localStorage`, storing each value as a JSON array
/// of byte values.
///
/// ## Where it stores things
///
/// `localStorage` is shared with everything else on the origin, so `prefix`
/// keeps this store's keys apart from the rest. It is empty by default, which
/// keeps a key the app already stores readable; give it one — `"myapp."` —
/// before the app has anything worth keeping.
///
/// ## When there is no `localStorage`
///
/// There is none on a server, in a worker, or when the browser has storage
/// turned off. Rather than pretend, every operation then answers
/// `keyValueErrorIo`, so the core sees a store that is not working rather than
/// one that is empty. Pass a `Storage` of your own — `sessionStorage`, or an
/// in-memory stand-in in a test — to decide otherwise.
///
/// @param prefix put in front of every key in the underlying storage.
/// @param storage the storage to use; the global `localStorage` when omitted,
///   read at call time so that a page can install one first.
export function createLocalStorageKeyValueHandler(
  prefix = "",
  storage?: Storage,
): KeyValueHandler {
  const open = (): Storage | undefined => {
    if (storage !== undefined) return storage;
    try {
      return globalThis.localStorage;
    } catch {
      // Reading the property itself throws when storage is blocked.
      return undefined;
    }
  };
  const unavailable = keyValueErrorIo("localStorage is not available");

  const read = (store: Storage, key: string): Value => {
    const stored = store.getItem(prefix + key);
    if (stored === null) return valueNone();
    return valueBytes(JSON.parse(stored) as number[]);
  };

  return {
    async get(operation: Get): Promise<ValueResult> {
      const store = open();
      if (store === undefined) return valueResultErr(unavailable);
      return valueResultOk(read(store, operation.key));
    },

    async set(operation: Set): Promise<ValueResult> {
      const store = open();
      if (store === undefined) return valueResultErr(unavailable);
      const previous = read(store, operation.key);
      try {
        store.setItem(
          prefix + operation.key,
          JSON.stringify(Array.from(operation.value)),
        );
      } catch (error) {
        // Over quota, most likely.
        return valueResultErr(
          keyValueErrorIo(error instanceof Error ? error.message : String(error)),
        );
      }
      return valueResultOk(previous);
    },

    async delete(operation: Delete): Promise<ValueResult> {
      const store = open();
      if (store === undefined) return valueResultErr(unavailable);
      const previous = read(store, operation.key);
      store.removeItem(prefix + operation.key);
      return valueResultOk(previous);
    },

    async exists(operation: Exists): Promise<BoolResult> {
      const store = open();
      if (store === undefined) return boolResultErr(unavailable);
      return boolResultOk(store.getItem(prefix + operation.key) !== null);
    },

    /// One page holds every remaining key, so the answer's cursor is always 0:
    /// `localStorage` is walked in one go anyway, and paging it would only
    /// invite a caller to hold a cursor across a write.
    async listKeys(operation: ListKeys): Promise<KeysResult> {
      const store = open();
      if (store === undefined) return keysResultErr(unavailable);

      const keys: string[] = [];
      for (let i = 0; i < store.length; i++) {
        const stored = store.key(i);
        if (stored === null || !stored.startsWith(prefix)) continue;
        const key = stored.slice(prefix.length);
        if (key.startsWith(operation.prefix)) keys.push(key);
      }
      keys.sort();

      const cursor = Number(operation.cursor);
      if (cursor > keys.length) return keysResultErr(keyValueErrorCursorNotFound());

      return keysResultOk(new KeyPage(keys.slice(cursor), BigInt(0)));
    },
  };
}

/// The plain path: one handler over the global `localStorage`, with no prefix.
export const localStorageKeyValueHandler: KeyValueHandler =
  createLocalStorageKeyValueHandler();
