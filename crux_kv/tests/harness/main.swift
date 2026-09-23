import App
import Foundation

/// Runs `UserDefaultsKeyValueHandler` through the whole `crux_kv` protocol and
/// prints `HARNESS OK` when every answer was the one the protocol promises.
///
/// The store is a suite of its own, named after a fresh UUID and removed at the
/// end, so a run cannot see or disturb anything else on the machine.

// Every mismatch, so that one run reports all of them rather than the first.
var failures: [String] = []

func expect(_ what: String, _ satisfied: Bool) {
    if !satisfied { failures.append(what) }
}

func expect<T: Equatable>(_ what: String, _ actual: T, _ expected: T) {
    if actual != expected { failures.append("\(what): expected \(expected), got \(actual)") }
}

/// The bytes a `ValueResult` carries, `nil` for `Value.none`.
///
/// An `.err` is a crash rather than a failure: every operation here is one the
/// protocol says cannot fail short of the store itself failing, so an error
/// means the run has nothing left to tell us.
func bytes(_ result: ValueResult) -> [UInt8]? {
    guard case .ok(let value) = result else { fatalError("expected .ok, got \(result)") }
    guard case .bytes(let bytes) = value else { return nil }
    return bytes
}

func keys(_ result: KeysResult) -> [String] {
    guard case .ok(let page) = result else { fatalError("expected .ok, got \(result)") }
    expect("listKeys answers with a cursor of 0, there being no more pages", page.nextCursor, 0)
    return page.keys
}

func present(_ result: ExistsResult) -> Bool {
    guard case .ok(let present) = result else { fatalError("expected .ok, got \(result)") }
    return present
}

/// A `/`, a space and a `%`, the three characters a store that names files
/// after keys has to encode and decode again.
let awkward = "alpha/two words%25"
/// Already in the order `listKeys` promises, so that the expected listing is
/// this list and not a sort of it.
let written = ["alpha", awkward, "beta", "delta"]

let suiteName = "com.redbadger.crux.kv.harness.\(UUID().uuidString)"
let defaults = UserDefaults(suiteName: suiteName)!
let kv = UserDefaultsKeyValueHandler(suiteName: suiteName)

// An empty store, before anything has been written to it.
expect("get of a missing key is Value.none, not an error", bytes(await kv.get(GetValue(key: "alpha"))) == nil)
expect("exists is false for a missing key", present(await kv.exists(KeyExists(key: "alpha"))), false)
expect(
    "listKeys of an empty store answers with no keys",
    keys(await kv.listKeys(ListKeys(prefix: "", cursor: 0))),
    []
)
expect(
    "delete of a missing key is Value.none, not an error",
    bytes(await kv.delete(DeleteValue(key: "alpha"))) == nil
)

// Each key holds its own name, so a value that comes back under the wrong key
// is visible rather than plausible.
for key in written {
    expect(
        "set of a new key answers with Value.none",
        bytes(await kv.set(SetValue(key: key, value: [UInt8](key.utf8)))) == nil
    )
}

for key in written {
    expect("get returns the bytes set under \(key)", bytes(await kv.get(GetValue(key: key))), [UInt8](key.utf8))
    expect("exists is true for \(key)", present(await kv.exists(KeyExists(key: key))), true)
}

expect(
    "set of an existing key answers with the value it replaced",
    bytes(await kv.set(SetValue(key: "beta", value: [9, 9]))),
    [UInt8]("beta".utf8)
)
expect("get returns the replacement", bytes(await kv.get(GetValue(key: "beta"))), [9, 9])

// The regression: everything the store lists is a key the app wrote. A
// `UserDefaults` read goes through the whole search list, so listing what
// `dictionaryRepresentation()` holds would answer with the global domain's
// keys as well — `AppleLanguages` and some sixty more.
expect(
    "listKeys answers with the keys that were written, sorted, and nothing else",
    keys(await kv.listKeys(ListKeys(prefix: "", cursor: 0))),
    written
)
expect(
    "listKeys honours the prefix",
    keys(await kv.listKeys(ListKeys(prefix: "alpha", cursor: 0))),
    ["alpha", awkward]
)
expect(
    "listKeys starts at the cursor",
    keys(await kv.listKeys(ListKeys(prefix: "", cursor: 2))),
    ["beta", "delta"]
)
expect(
    "a cursor equal to the number of keys is an empty page, not an error",
    keys(await kv.listKeys(ListKeys(prefix: "", cursor: UInt64(written.count)))),
    []
)
if case .err(.cursorNotFound) = await kv.listKeys(ListKeys(prefix: "", cursor: UInt64(written.count + 1))) {
} else {
    failures.append("a cursor past the end answers with CursorNotFound")
}

expect(
    "the awkward key survives the round trip",
    bytes(await kv.get(GetValue(key: awkward))),
    [UInt8](awkward.utf8)
)

expect(
    "delete answers with the value it removed",
    bytes(await kv.delete(DeleteValue(key: "delta"))),
    [UInt8]("delta".utf8)
)
expect("exists is false once the key is deleted", present(await kv.exists(KeyExists(key: "delta"))), false)
expect("get is Value.none once the key is deleted", bytes(await kv.get(GetValue(key: "delta"))) == nil)
expect(
    "listKeys no longer answers with the deleted key",
    keys(await kv.listKeys(ListKeys(prefix: "", cursor: 0))),
    ["alpha", awkward, "beta"]
)

// A preference the app put in the same defaults is not a value this store
// holds: `get` answers `Value.none` for it, and `exists` has to agree.
defaults.set("a preference, not a stored value", forKey: "zeta")
expect("get of a key whose value is not Data is Value.none", bytes(await kv.get(GetValue(key: "zeta"))) == nil)
expect("exists agrees with get about a key whose value is not Data", present(await kv.exists(KeyExists(key: "zeta"))), false)

UserDefaults.standard.removePersistentDomain(forName: suiteName)

if failures.isEmpty {
    print("HARNESS OK")
} else {
    for failure in failures { print("FAILED: \(failure)") }
    exit(1)
}
