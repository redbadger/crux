import type { KeyValueHandler, ValueResult } from "shared_types/app";
import {
  existsResultOk,
  KeyPage,
  keysResultErr,
  keysResultOk,
  keyValueErrorCursorNotFound,
  keyValueErrorIo,
  keyValueErrorOther,
  valueBytes,
  valueNone,
  valueResultErr,
  valueResultOk,
} from "shared_types/app";

/**
 * A `KeyValueHandler` over the browser's `localStorage`. Each value is stored
 * as a JSON array of bytes.
 *
 * `localStorage` is shared with everything else on the origin, so `prefix`
 * keeps the keys of this store apart from other keys.
 */
export function createLocalStorageKeyValueHandler(
  prefix = "",
): KeyValueHandler {
  const read = (key: string): ValueResult => {
    const stored = localStorage.getItem(prefix + key);
    if (stored === null) return valueResultOk(valueNone());
    try {
      return valueResultOk(valueBytes(JSON.parse(stored) as number[]));
    } catch {
      return valueResultErr(keyValueErrorOther(`"${key}" is not a byte array`));
    }
  };

  return {
    async get({ key }) {
      return read(key);
    },

    async set({ key, value }) {
      const previous = read(key);
      try {
        localStorage.setItem(prefix + key, JSON.stringify(Array.from(value)));
      } catch (error) {
        // The browser throws when the origin is over its storage quota.
        return valueResultErr(keyValueErrorIo(String(error)));
      }
      return previous;
    },

    async delete({ key }) {
      const previous = read(key);
      localStorage.removeItem(prefix + key);
      return previous;
    },

    async exists({ key }) {
      return existsResultOk(localStorage.getItem(prefix + key) !== null);
    },

    async listKeys({ prefix: start, cursor }) {
      const keys = Object.keys(localStorage)
        .filter((stored) => stored.startsWith(prefix + start))
        .map((stored) => stored.slice(prefix.length))
        .sort();
      const from = Number(cursor);
      if (from > keys.length) {
        return keysResultErr(keyValueErrorCursorNotFound());
      }
      return keysResultOk(new KeyPage(keys.slice(from), BigInt(0)));
    },
  };
}
