import com.example.shared.BoolResult
import com.example.shared.Delete
import com.example.shared.Exists
import com.example.shared.FileKeyValueHandler
import com.example.shared.Get
import com.example.shared.KeyValueError
import com.example.shared.KeysResult
import com.example.shared.ListKeys
import com.example.shared.Set
import com.example.shared.Value
import com.example.shared.ValueResult
import kotlinx.coroutines.runBlocking
import java.io.File
import kotlin.system.exitProcess

/// Runs `FileKeyValueHandler` through the whole `crux_kv` protocol and prints
/// `HARNESS OK` when every answer was the one the protocol promises.
///
/// The store is a directory that does not exist when the handler is built, so
/// the run also covers the first write to a fresh store.

// Every mismatch, so that one run reports all of them rather than the first.
val failures = mutableListOf<String>()

fun expect(
    what: String,
    satisfied: Boolean,
) {
    if (!satisfied) failures.add(what)
}

fun <T> expect(
    what: String,
    actual: T,
    expected: T,
) {
    if (actual != expected) failures.add("$what: expected $expected, got $actual")
}

/// The bytes a [ValueResult] carries, `null` for [Value.None].
///
/// An `Err` is a crash rather than a failure: every operation here is one the
/// protocol says cannot fail short of the store itself failing, so an error
/// means the run has nothing left to tell us.
fun bytes(result: ValueResult): List<UByte>? =
    when (result) {
        is ValueResult.Ok ->
            when (val value = result.value) {
                is Value.Bytes -> value.value
                Value.None -> null
            }
        is ValueResult.Err -> error("expected Ok, got $result")
    }

fun keys(result: KeysResult): List<String> =
    when (result) {
        is KeysResult.Ok -> {
            expect("listKeys answers with a cursor of 0, there being no more pages", result.value.nextCursor, 0uL)
            result.value.keys
        }
        is KeysResult.Err -> error("expected Ok, got $result")
    }

fun present(result: BoolResult): Boolean =
    when (result) {
        is BoolResult.Ok -> result.value
        is BoolResult.Err -> error("expected Ok, got $result")
    }

fun value(key: String): List<UByte> = key.toByteArray(Charsets.UTF_8).map { it.toUByte() }

/// A `/`, a space and a `%`, the three characters a store that names files
/// after keys has to encode and decode again.
const val AWKWARD = "alpha/two words%25"

/// Already in the order `listKeys` promises, so that the expected listing is
/// this list and not a sort of it.
val written = listOf("alpha", AWKWARD, "beta", "delta")

fun main() =
    runBlocking {
        // A hung answer would otherwise wait for the test's own timeout, with
        // nothing to say for itself.
        Thread {
            Thread.sleep(60_000)
            System.err.println("timed out: an operation never answered")
            exitProcess(2)
        }.apply { isDaemon = true }.start()

        val root = File(System.getProperty("java.io.tmpdir"), "crux-kv-harness-${System.nanoTime()}")
        val store = File(root, "store")
        val kv = FileKeyValueHandler(store)

        // A store whose directory does not exist yet.
        expect("the store's directory does not exist yet", !store.exists())
        expect("get of a missing key is Value.None, not an error", bytes(kv.get(Get("alpha"))) == null)
        expect("exists is false for a missing key", present(kv.exists(Exists("alpha"))), false)
        expect("listKeys of an empty store answers with no keys", keys(kv.listKeys(ListKeys("", 0uL))), emptyList())
        expect("delete of a missing key is Value.None, not an error", bytes(kv.delete(Delete("alpha"))) == null)

        // Each key holds its own name, so a value that comes back under the
        // wrong key is visible rather than plausible.
        for (key in written) {
            expect("set of a new key answers with Value.None", bytes(kv.set(Set(key, value(key)))) == null)
        }

        for (key in written) {
            expect("get returns the bytes set under $key", bytes(kv.get(Get(key))), value(key))
            expect("exists is true for $key", present(kv.exists(Exists(key))), true)
        }

        expect(
            "set of an existing key answers with the value it replaced",
            bytes(kv.set(Set("beta", listOf<UByte>(9u, 9u)))),
            value("beta"),
        )
        expect("get returns the replacement", bytes(kv.get(Get("beta"))), listOf<UByte>(9u, 9u))

        // The regression: everything the store lists is a key the app wrote.
        // The store holds a directory of half-written values beside the keys,
        // and a directory is not a key; nor is anything else the store keeps
        // for itself.
        File(store, "not-a-key-either").mkdirs()
        expect(
            "listKeys answers with the keys that were written, sorted, and nothing else",
            keys(kv.listKeys(ListKeys("", 0uL))),
            written,
        )
        expect("listKeys honours the prefix", keys(kv.listKeys(ListKeys("alpha", 0uL))), listOf("alpha", AWKWARD))
        expect("listKeys starts at the cursor", keys(kv.listKeys(ListKeys("", 2uL))), listOf("beta", "delta"))
        expect(
            "a cursor equal to the number of keys is an empty page, not an error",
            keys(kv.listKeys(ListKeys("", written.size.toULong()))),
            emptyList(),
        )
        expect(
            "a cursor past the end answers with CursorNotFound",
            kv.listKeys(ListKeys("", written.size.toULong() + 1uL)) == KeysResult.Err(KeyValueError.CursorNotFound),
        )

        expect("the awkward key survives the round trip", bytes(kv.get(Get(AWKWARD))), value(AWKWARD))

        expect("delete answers with the value it removed", bytes(kv.delete(Delete("delta"))), value("delta"))
        expect("exists is false once the key is deleted", present(kv.exists(Exists("delta"))), false)
        expect("get is Value.None once the key is deleted", bytes(kv.get(Get("delta"))) == null)
        expect(
            "listKeys no longer answers with the deleted key",
            keys(kv.listKeys(ListKeys("", 0uL))),
            listOf("alpha", AWKWARD, "beta"),
        )

        root.deleteRecursively()

        if (failures.isEmpty()) {
            println("HARNESS OK")
        } else {
            failures.forEach { println("FAILED: $it") }
            exitProcess(1)
        }
    }
