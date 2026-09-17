// Resolve secret requests using localStorage.
//
// Web browsers don't have a secure secrets store, so localStorage is the
// closest available approximation. Values are stored in plaintext and
// accessible to any script on the same origin.
//
// Each operation has its own response type, naming only the outcomes it can
// actually produce — there is no wide response enum to narrow.

import type {
  DeleteSecret,
  FetchSecret,
  SecretDeleteResponse,
  SecretFetchResponse,
  SecretStoreResponse,
  StoreSecret,
} from "shared_types/app";
import {
  secretDeleteResponseDeleted,
  secretFetchResponseFetched,
  secretFetchResponseMissing,
  secretStoreResponseStored,
} from "shared_types/app";

export async function fetch(
  operation: FetchSecret,
): Promise<SecretFetchResponse> {
  const key = operation.value;
  console.debug("secret fetch:", key);
  const value = localStorage.getItem(key);
  if (value !== null) {
    console.debug("secret found:", key);
    return secretFetchResponseFetched(value);
  }
  console.debug("secret not found:", key);
  return secretFetchResponseMissing(key);
}

export async function store(
  operation: StoreSecret,
): Promise<SecretStoreResponse> {
  const { field0: key, field1: value } = operation;
  console.debug("secret store:", key);
  localStorage.setItem(key, value);
  return secretStoreResponseStored(key);
}

export async function remove(
  operation: DeleteSecret,
): Promise<SecretDeleteResponse> {
  const key = operation.value;
  console.debug("secret delete:", key);
  localStorage.removeItem(key);
  return secretDeleteResponseDeleted(key);
}
