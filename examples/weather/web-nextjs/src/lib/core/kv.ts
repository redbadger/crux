import type { Get, Set, ValueResult } from "shared_types/app";
import { valueBytes, valueResultOk } from "shared_types/app";

export async function get(operation: Get): Promise<ValueResult> {
  console.debug("kv get:", operation.key);
  return valueResultOk(read(operation.key));
}

export async function set(operation: Set): Promise<ValueResult> {
  console.debug("kv set:", operation.key);
  const previous = read(operation.key);
  const value = new TextDecoder().decode(new Uint8Array(operation.value));
  localStorage.setItem(operation.key, value);
  return valueResultOk(previous);
}

function read(key: string) {
  const stored = localStorage.getItem(key);
  return valueBytes(stored ? Array.from(new TextEncoder().encode(stored)) : []);
}
