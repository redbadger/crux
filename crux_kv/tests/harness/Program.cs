// Runs `FileKeyValueHandler` through the whole `crux_kv` protocol and prints
// `HARNESS OK` when every answer was the one the protocol promises.
//
// The store is a directory that does not exist when the handler is built, so
// the run also covers the first write to a fresh store.

using System.Collections.ObjectModel;
using System.Text;
using Example.Shared;

// Every mismatch, so that one run reports all of them rather than the first.
var failures = new List<string>();

void Expect<T>(string what, T actual, T expected)
{
    if (!Show(actual).Equals(Show(expected), StringComparison.Ordinal))
    {
        failures.Add($"{what}: expected {Show(expected)}, got {Show(actual)}");
    }
}

static string Show<T>(T value) =>
    value switch
    {
        null => "null",
        IEnumerable<byte> bytes => "[" + string.Join(",", bytes) + "]",
        IEnumerable<string> strings => "[" + string.Join(",", strings) + "]",
        _ => value.ToString() ?? "null",
    };

// The bytes a `ValueResult` carries, `null` for `Value.None`.
//
// An `Err` is a crash rather than a failure: every operation here is one the
// protocol says cannot fail short of the store itself failing, so an error
// means the run has nothing left to tell us.
static byte[]? Bytes(ValueResult result) =>
    result switch
    {
        ValueResult.Ok(Value.Bytes bytes) => [.. bytes.Value],
        ValueResult.Ok(Value.None) => null,
        _ => throw new InvalidOperationException($"expected Ok, got {result}"),
    };

List<string> Keys(KeysResult result)
{
    if (result is not KeysResult.Ok ok)
    {
        throw new InvalidOperationException($"expected Ok, got {result}");
    }
    Expect("listKeys answers with a cursor of 0, there being no more pages", ok.Value.NextCursor, 0UL);
    return [.. ok.Value.Keys];
}

static bool Present(BoolResult result) =>
    result switch
    {
        BoolResult.Ok ok => ok.Value,
        _ => throw new InvalidOperationException($"expected Ok, got {result}"),
    };

// Named `Stored`, not `Value`, because `Value` is a generated type.
static byte[] Stored(string key) => Encoding.UTF8.GetBytes(key);

// A `/`, a space and a `%`, the three characters a store that names files
// after keys has to encode and decode again.
const string Awkward = "alpha/two words%25";

// Already in the order `ListKeys` promises, so that the expected listing is
// this list and not a sort of it.
string[] written = ["alpha", Awkward, "beta", "delta"];

var root = Path.Combine(Path.GetTempPath(), $"crux-kv-harness-{Guid.NewGuid():N}");
var store = Path.Combine(root, "store");
IKeyValueHandler kv = new FileKeyValueHandler(store);

// A store whose directory does not exist yet.
Expect("the store's directory does not exist yet", Directory.Exists(store), false);
Expect("Get of a missing key is Value.None, not an error", Bytes(await kv.Get(new Get { Key = "alpha" })), null);
Expect("Exists is false for a missing key", Present(await kv.Exists(new Exists { Key = "alpha" })), false);
Expect(
    "ListKeys of an empty store answers with no keys",
    Keys(await kv.ListKeys(new ListKeys { Prefix = "", Cursor = 0 })),
    []
);
Expect(
    "Delete of a missing key is Value.None, not an error",
    Bytes(await kv.Delete(new Delete { Key = "alpha" })),
    null
);

// Each key holds its own name, so a value that comes back under the wrong key
// is visible rather than plausible.
foreach (var key in written)
{
    var set = new Set { Key = key, Value = new ObservableCollection<byte>(Stored(key)) };
    Expect("Set of a new key answers with Value.None", Bytes(await kv.Set(set)), null);
}

foreach (var key in written)
{
    Expect($"Get returns the bytes set under {key}", Bytes(await kv.Get(new Get { Key = key })), Stored(key));
    Expect($"Exists is true for {key}", Present(await kv.Exists(new Exists { Key = key })), true);
}

Expect(
    "Set of an existing key answers with the value it replaced",
    Bytes(await kv.Set(new Set { Key = "beta", Value = [9, 9] })),
    Stored("beta")
);
Expect("Get returns the replacement", Bytes(await kv.Get(new Get { Key = "beta" })), [9, 9]);

// The regression: everything the store lists is a key the app wrote. The store
// holds a directory of half-written values beside the keys, and a directory is
// not a key; nor is anything else the store keeps for itself.
Directory.CreateDirectory(Path.Combine(store, "not-a-key-either"));
Expect(
    "ListKeys answers with the keys that were written, sorted, and nothing else",
    Keys(await kv.ListKeys(new ListKeys { Prefix = "", Cursor = 0 })),
    [.. written]
);
Expect(
    "ListKeys honours the prefix",
    Keys(await kv.ListKeys(new ListKeys { Prefix = "alpha", Cursor = 0 })),
    ["alpha", Awkward]
);
Expect(
    "ListKeys starts at the cursor",
    Keys(await kv.ListKeys(new ListKeys { Prefix = "", Cursor = 2 })),
    ["beta", "delta"]
);
Expect(
    "a cursor equal to the number of keys is an empty page, not an error",
    Keys(await kv.ListKeys(new ListKeys { Prefix = "", Cursor = (ulong)written.Length })),
    []
);
Expect(
    "a cursor past the end answers with CursorNotFound",
    await kv.ListKeys(new ListKeys { Prefix = "", Cursor = (ulong)written.Length + 1 }),
    new KeysResult.Err(new KeyValueError.CursorNotFound())
);

Expect("the awkward key survives the round trip", Bytes(await kv.Get(new Get { Key = Awkward })), Stored(Awkward));

Expect(
    "Delete answers with the value it removed",
    Bytes(await kv.Delete(new Delete { Key = "delta" })),
    Stored("delta")
);
Expect("Exists is false once the key is deleted", Present(await kv.Exists(new Exists { Key = "delta" })), false);
Expect("Get is Value.None once the key is deleted", Bytes(await kv.Get(new Get { Key = "delta" })), null);
Expect(
    "ListKeys no longer answers with the deleted key",
    Keys(await kv.ListKeys(new ListKeys { Prefix = "", Cursor = 0 })),
    ["alpha", Awkward, "beta"]
);

Directory.Delete(root, recursive: true);

if (failures.Count == 0)
{
    Console.WriteLine("HARNESS OK");
    return 0;
}

foreach (var failure in failures)
{
    Console.WriteLine($"FAILED: {failure}");
}
return 1;
