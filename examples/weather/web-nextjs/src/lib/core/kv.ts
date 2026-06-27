import type { KeyValueOperation, KeyValueResult } from "shared_types/app";
import {
  matchKeyValueOperation,
  keyValueResultOk,
  keyValueResponseGet,
  keyValueResponseSet,
  keyValueResponseDelete,
  keyValueResponseExists,
  keyValueResponseListKeys,
  valueBytes,
} from "shared_types/app";

export async function handle(
  operation: KeyValueOperation,
): Promise<KeyValueResult> {
  return matchKeyValueOperation(operation, {
    Get: (op) => {
      const key = op.key;
      console.debug("kv get:", key);
      const stored = localStorage.getItem(key);
      const bytes = stored ? Array.from(new TextEncoder().encode(stored)) : [];
      return Promise.resolve(
        keyValueResultOk(keyValueResponseGet(valueBytes(bytes))),
      );
    },
    Set: (op) => {
      const { key, value } = op;
      console.debug("kv set:", key);
      const previous = localStorage.getItem(key);
      const prevBytes = previous
        ? Array.from(new TextEncoder().encode(previous))
        : [];
      const valueStr = new TextDecoder().decode(new Uint8Array(value));
      localStorage.setItem(key, valueStr);
      return Promise.resolve(
        keyValueResultOk(keyValueResponseSet(valueBytes(prevBytes))),
      );
    },
    Delete: (op) => {
      const key = op.key;
      console.debug("kv delete:", key);
      const previous = localStorage.getItem(key);
      const prevBytes = previous
        ? Array.from(new TextEncoder().encode(previous))
        : [];
      localStorage.removeItem(key);
      return Promise.resolve(
        keyValueResultOk(keyValueResponseDelete(valueBytes(prevBytes))),
      );
    },
    Exists: (op) => {
      const key = op.key;
      const exists = localStorage.getItem(key) !== null;
      console.debug("kv exists:", key, exists);
      return Promise.resolve(keyValueResultOk(keyValueResponseExists(exists)));
    },
    ListKeys: (op) => {
      const { prefix } = op;
      console.debug("kv list_keys: prefix=", prefix);
      const keys: string[] = [];
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key && key.startsWith(prefix)) {
          keys.push(key);
        }
      }
      return Promise.resolve(
        keyValueResultOk(keyValueResponseListKeys(keys, BigInt(0))),
      );
    },
  });
}
