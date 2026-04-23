// Resolve a secret request using localStorage.
//
// Web browsers don't have a secure secrets store, so localStorage is the
// closest available approximation. Values are stored in plaintext and
// accessible to any script on the same origin.

import type { SecretRequest, SecretResponse } from "shared_types/app";
import {
  SecretRequestVariantFetch,
  SecretRequestVariantStore,
  SecretRequestVariantDelete,
  SecretResponseVariantMissing,
  SecretResponseVariantFetched,
  SecretResponseVariantStored,
  SecretResponseVariantDeleted,
} from "shared_types/app";

export function handle(request: SecretRequest): SecretResponse {
  if (request instanceof SecretRequestVariantFetch) {
    const key = request.value;
    console.debug("secret fetch:", key);
    const value = localStorage.getItem(key);
    if (value !== null) {
      console.debug("secret found:", key);
      return new SecretResponseVariantFetched(key, value);
    }
    console.debug("secret not found:", key);
    return new SecretResponseVariantMissing(key);
  }

  if (request instanceof SecretRequestVariantStore) {
    const key = request.field0;
    const value = request.field1;
    console.debug("secret store:", key);
    localStorage.setItem(key, value);
    return new SecretResponseVariantStored(key);
  }

  if (request instanceof SecretRequestVariantDelete) {
    const key = request.value;
    console.debug("secret delete:", key);
    localStorage.removeItem(key);
    return new SecretResponseVariantDeleted(key);
  }

  throw new Error(`Unhandled secret operation: ${request.constructor.name}`);
}
