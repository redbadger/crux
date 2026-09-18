// Runs `createLocalStorageKeyValueHandler` through the whole `crux_kv`
// protocol and prints `HARNESS OK` when every answer was the one the protocol
// promises.
//
// The generated module is CommonJS — `typescript()` emits `.js` beside the
// `.d.ts` it type-checks — so the harness is plain JavaScript and `node` runs
// it with no build step of its own. The types are already checked by the
// compile test above; what is left to find out is what the code does.
//
// There is no `localStorage` in node, and the handler takes a `Storage` for
// exactly that reason, so the store here is an in-memory one. That is the
// substitution its own documentation recommends for a test, and it leaves the
// prefixing, listing, paging and encoding — everything the handler itself
// does — under test.

const shared = require("../generated/shared_types.js");

/// Every mismatch, so that one run reports all of them rather than the first.
const failures = [];

/// `JSON.stringify` refuses a `BigInt`, and the cursor is one.
const show = (value) => JSON.stringify(value, (_, v) => (typeof v === "bigint" ? `${v}n` : v));

const expect = (what, actual, expected) => {
  const got = show(actual);
  const wanted = show(expected);
  if (got !== wanted) failures.push(`${what}: expected ${wanted}, got ${got}`);
};

/// The smallest `Storage` the handler will accept, holding strings in a `Map`.
class MemoryStorage {
  constructor() {
    this.entries = new Map();
  }

  get length() {
    return this.entries.size;
  }

  key(index) {
    const keys = [...this.entries.keys()];
    return index < keys.length ? keys[index] : null;
  }

  getItem(key) {
    const value = this.entries.get(key);
    return value === undefined ? null : value;
  }

  setItem(key, value) {
    this.entries.set(key, String(value));
  }

  removeItem(key) {
    this.entries.delete(key);
  }

  clear() {
    this.entries.clear();
  }
}

/// The bytes a `ValueResult` carries, `null` for `Value.None`.
///
/// An `Err` is a crash rather than a failure: every operation here is one the
/// protocol says cannot fail short of the store itself failing, so an error
/// means the run has nothing left to tell us.
const bytes = (result) => {
  if (result.kind !== "Ok") throw new Error(`expected Ok, got ${show(result)}`);
  return result.value.kind === "Bytes" ? result.value.value : null;
};

const keys = (result) => {
  if (result.kind !== "Ok") throw new Error(`expected Ok, got ${show(result)}`);
  expect("listKeys answers with a cursor of 0, there being no more pages", result.value.next_cursor, 0n);
  return result.value.keys;
};

const present = (result) => {
  if (result.kind !== "Ok") throw new Error(`expected Ok, got ${show(result)}`);
  return result.value;
};

const value = (key) => [...Buffer.from(key, "utf8")];

/// A `/`, a space and a `%`, the three characters a store that names files
/// after keys has to encode and decode again. `localStorage` does not, but the
/// key has to survive a round trip all the same.
const awkward = "alpha/two words%25";

/// Already in the order `listKeys` promises, so that the expected listing is
/// this list and not a sort of it.
const written = ["alpha", awkward, "beta", "delta"];

const main = async () => {
  const storage = new MemoryStorage();
  // Something else on the origin, under no prefix of this store's, which
  // `listKeys` must not answer with.
  storage.setItem("someone-elses-key", "not ours");

  const kv = shared.createLocalStorageKeyValueHandler("harness.", storage);

  // An empty store, before anything has been written to it.
  expect("get of a missing key is Value.None, not an error", bytes(await kv.get(new shared.Get("alpha"))), null);
  expect("exists is false for a missing key", present(await kv.exists(new shared.Exists("alpha"))), false);
  expect("listKeys of an empty store answers with no keys", keys(await kv.listKeys(new shared.ListKeys("", 0n))), []);
  expect(
    "delete of a missing key is Value.None, not an error",
    bytes(await kv.delete(new shared.Delete("alpha"))),
    null,
  );

  // Each key holds its own name, so a value that comes back under the wrong
  // key is visible rather than plausible.
  for (const key of written) {
    expect("set of a new key answers with Value.None", bytes(await kv.set(new shared.Set(key, value(key)))), null);
  }

  for (const key of written) {
    expect(`get returns the bytes set under ${key}`, bytes(await kv.get(new shared.Get(key))), value(key));
    expect(`exists is true for ${key}`, present(await kv.exists(new shared.Exists(key))), true);
  }

  expect(
    "set of an existing key answers with the value it replaced",
    bytes(await kv.set(new shared.Set("beta", [9, 9]))),
    value("beta"),
  );
  expect("get returns the replacement", bytes(await kv.get(new shared.Get("beta"))), [9, 9]);

  // The regression: everything the store lists is a key the app wrote, and
  // nothing the store shares its storage with.
  expect(
    "listKeys answers with the keys that were written, sorted, and nothing else",
    keys(await kv.listKeys(new shared.ListKeys("", 0n))),
    written,
  );
  expect("listKeys honours the prefix", keys(await kv.listKeys(new shared.ListKeys("alpha", 0n))), ["alpha", awkward]);
  expect("listKeys starts at the cursor", keys(await kv.listKeys(new shared.ListKeys("", 2n))), ["beta", "delta"]);
  expect(
    "a cursor equal to the number of keys is an empty page, not an error",
    keys(await kv.listKeys(new shared.ListKeys("", BigInt(written.length)))),
    [],
  );
  expect(
    "a cursor past the end answers with CursorNotFound",
    await kv.listKeys(new shared.ListKeys("", BigInt(written.length + 1))),
    shared.keysResultErr(shared.keyValueErrorCursorNotFound()),
  );

  expect("the awkward key survives the round trip", bytes(await kv.get(new shared.Get(awkward))), value(awkward));

  expect("delete answers with the value it removed", bytes(await kv.delete(new shared.Delete("delta"))), value("delta"));
  expect("exists is false once the key is deleted", present(await kv.exists(new shared.Exists("delta"))), false);
  expect("get is Value.None once the key is deleted", bytes(await kv.get(new shared.Get("delta"))), null);
  expect("listKeys no longer answers with the deleted key", keys(await kv.listKeys(new shared.ListKeys("", 0n))), [
    "alpha",
    awkward,
    "beta",
  ]);

  expect("the store leaves the storage it shares alone", storage.getItem("someone-elses-key"), "not ours");
};

main().then(() => {
  if (failures.length === 0) {
    console.log("HARNESS OK");
  } else {
    for (const failure of failures) console.log(`FAILED: ${failure}`);
    process.exitCode = 1;
  }
});
