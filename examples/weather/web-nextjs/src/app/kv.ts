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
      const key = (operation as KeyValueOperationVariantGet).value;
      const stored = localStorage.getItem(key);
      const bytes = stored
        ? new TextEncoder().encode(stored)
        : new Uint8Array();
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantGet(new ValueVariantBytes(bytes)),
      );
    }
    case KeyValueOperationVariantSet: {
      const { field0: key, field1: value } =
        operation as KeyValueOperationVariantSet;
      const previous = localStorage.getItem(key);
      const prevBytes = previous
        ? new TextEncoder().encode(previous)
        : new Uint8Array();
      const valueStr = new TextDecoder().decode(new Uint8Array(value));
      localStorage.setItem(key, valueStr);
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantSet(new ValueVariantBytes(prevBytes)),
      );
    }
    case KeyValueOperationVariantDelete: {
      const key = (operation as KeyValueOperationVariantDelete).value;
      const previous = localStorage.getItem(key);
      const prevBytes = previous
        ? new TextEncoder().encode(previous)
        : new Uint8Array();
      localStorage.removeItem(key);
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantDelete(new ValueVariantBytes(prevBytes)),
      );
    }
    case KeyValueOperationVariantExists: {
      const key = (operation as KeyValueOperationVariantExists).value;
      const exists = localStorage.getItem(key) !== null;
      return new KeyValueResultVariantOk(
        new KeyValueResponseVariantExists(exists),
      );
    }
    case KeyValueOperationVariantListKeys: {
      const { field0: prefix } =
        operation as KeyValueOperationVariantListKeys;
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
