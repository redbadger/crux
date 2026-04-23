import type { KeyValueOperation, KeyValueResult } from "shared_types/app";
import {
  KeyValueOperationVariantGet,
  KeyValueOperationVariantSet,
  KeyValueOperationVariantDelete,
  KeyValueOperationVariantExists,
  KeyValueOperationVariantListKeys,
  KeyValueResultVariantOk,
  KeyValueResponseVariantGet,
  KeyValueResponseVariantSet,
  KeyValueResponseVariantDelete,
  KeyValueResponseVariantExists,
  KeyValueResponseVariantListKeys,
  ValueVariantBytes,
} from "shared_types/app";

export async function handle(
  operation: KeyValueOperation,
): Promise<KeyValueResult> {
  switch (operation.constructor) {
    case KeyValueOperationVariantGet: {
      const key = (operation as KeyValueOperationVariantGet).key;
      console.debug("kv get:", key);
      const stored = localStorage.getItem(key);
      const bytes = stored
        ? Array.from(new TextEncoder().encode(stored))
        : [];
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantGet(new ValueVariantBytes(bytes)),
      );
    }
    case KeyValueOperationVariantSet: {
      const { key, value } = operation as KeyValueOperationVariantSet;
      console.debug("kv set:", key);
      const previous = localStorage.getItem(key);
      const prevBytes = previous
        ? Array.from(new TextEncoder().encode(previous))
        : [];
      const valueStr = new TextDecoder().decode(new Uint8Array(value));
      localStorage.setItem(key, valueStr);
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantSet(new ValueVariantBytes(prevBytes)),
      );
    }
    case KeyValueOperationVariantDelete: {
      const key = (operation as KeyValueOperationVariantDelete).key;
      console.debug("kv delete:", key);
      const previous = localStorage.getItem(key);
      const prevBytes = previous
        ? Array.from(new TextEncoder().encode(previous))
        : [];
      localStorage.removeItem(key);
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantDelete(new ValueVariantBytes(prevBytes)),
      );
    }
    case KeyValueOperationVariantExists: {
      const key = (operation as KeyValueOperationVariantExists).key;
      const exists = localStorage.getItem(key) !== null;
      console.debug("kv exists:", key, exists);
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantExists(exists),
      );
    }
    case KeyValueOperationVariantListKeys: {
      const { prefix } = operation as KeyValueOperationVariantListKeys;
      console.debug("kv list_keys: prefix=", prefix);
      const keys: string[] = [];
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key && key.startsWith(prefix)) {
          keys.push(key);
        }
      }
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantListKeys(keys, BigInt(0)),
      );
    }
    default:
      throw new Error(`Unhandled KeyValue operation: ${operation.constructor}`);
  }
}
