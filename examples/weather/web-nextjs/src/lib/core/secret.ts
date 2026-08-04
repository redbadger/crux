// Resolve a secret request using localStorage.
//
// Web browsers don't have a secure secrets store, so localStorage is the
// closest available approximation. Values are stored in plaintext and
// accessible to any script on the same origin.

import type { SecretRequest, SecretResponse } from "shared_types/app";
import {
  matchSecretRequest,
  secretResponseMissing,
  secretResponseFetched,
  secretResponseStored,
  secretResponseDeleted,
} from "shared_types/app";

export function handle(request: SecretRequest): SecretResponse {
  return matchSecretRequest(request, {
    Fetch: (r) => {
      const key = r.value;
      console.debug("secret fetch:", key);
      const value = localStorage.getItem(key);
      if (value !== null) {
        console.debug("secret found:", key);
        return secretResponseFetched(key, value);
      }
      console.debug("secret not found:", key);
      return secretResponseMissing(key);
    },
    Store: (r) => {
      const key = r.field0;
      const value = r.field1;
      console.debug("secret store:", key);
      localStorage.setItem(key, value);
      return secretResponseStored(key);
    },
    Delete: (r) => {
      const key = r.value;
      console.debug("secret delete:", key);
      localStorage.removeItem(key);
      return secretResponseDeleted(key);
    },
  });
}
