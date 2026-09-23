import com.example.shared.DeleteValue
import com.example.shared.ExistsResult
import com.example.shared.GetValue
import com.example.shared.InMemoryKeyValueHandler
import com.example.shared.KeyExists
import com.example.shared.KeyValueError
import com.example.shared.KeyValueHandler
import com.example.shared.KeysResult
import com.example.shared.ListKeys
import com.example.shared.SetValue
import com.example.shared.Value
import com.example.shared.ValueResult
import kotlinx.coroutines.runBlocking
import kotlin.system.exitProcess

/**
 * Runs `InMemoryKeyValueHandler` through the whole `crux_kv` protocol and
 * prints `HARNESS OK` when every answer was the one the protocol promises.
 */

// Every mismatch, so that one run reports all of them rather than the first.
val failures = mutableListOf<String>()

/** The store under test, named in front of each mismatch. */
var store = ""

fun expect(
    what: String,
    satisfied: Boolean,
) {
    if (!satisfied) failures.add("$store: $what")
}

fun <T> expect(
    what: String,
    actual: T,
    expected: T,
) {
    if (actual != expected) failures.add("$store: $what: expected $expected, got $actual")
}

/**
 * The bytes a [ValueResult] carries, `null` for [Value.None].
 *
 * An `Err` is a crash rather than a failure: every operation here is one the
 * protocol says cannot fail short of the store itself failing, so an error
 * means the run has nothing left to tell us.
 */
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

fun present(result: ExistsResult): Boolean =
    when (result) {
        is ExistsResult.Ok -> result.value
        is ExistsResult.Err -> error("expected Ok, got $result")
    }

fun value(key: String): List<UByte> = key.toByteArray(Charsets.UTF_8).map { it.toUByte() }

/** A `/`, a space and a `%`, which a key has to survive a round trip with. */
const val AWKWARD = "alpha/two words%25"

/**
 * Already in the order `listKeys` promises, so that the expected listing is
 * this list and not a sort of it.
 */
val written = listOf("alpha", AWKWARD, "beta", "delta")

/**
 * The checks that hold for every store: the answers to each operation on an
 * empty store, then on the keys in [written].
 */
suspend fun check(kv: KeyValueHandler) {
    // An empty store, before anything has been written to it.
    expect("get of a missing key is Value.None, not an error", bytes(kv.get(GetValue("alpha"))) == null)
    expect("exists is false for a missing key", present(kv.exists(KeyExists("alpha"))), false)
    expect("listKeys of an empty store answers with no keys", keys(kv.listKeys(ListKeys("", 0uL))), emptyList())
    expect("delete of a missing key is Value.None, not an error", bytes(kv.delete(DeleteValue("alpha"))) == null)

    // Each key holds its own name, so a value that comes back under the wrong
    // key is visible rather than plausible.
    for (key in written) {
        expect("set of a new key answers with Value.None", bytes(kv.set(SetValue(key, value(key)))) == null)
    }

    for (key in written) {
        expect("get returns the bytes set under $key", bytes(kv.get(GetValue(key))), value(key))
        expect("exists is true for $key", present(kv.exists(KeyExists(key))), true)
    }

    expect(
        "set of an existing key answers with the value it replaced",
        bytes(kv.set(SetValue("beta", listOf<UByte>(9u, 9u)))),
        value("beta"),
    )
    expect("get returns the replacement", bytes(kv.get(GetValue("beta"))), listOf<UByte>(9u, 9u))

    // Everything the store lists is a key the app wrote.
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

    expect("the awkward key survives the round trip", bytes(kv.get(GetValue(AWKWARD))), value(AWKWARD))

    expect("delete answers with the value it removed", bytes(kv.delete(DeleteValue("delta"))), value("delta"))
    expect("exists is false once the key is deleted", present(kv.exists(KeyExists("delta"))), false)
    expect("get is Value.None once the key is deleted", bytes(kv.get(GetValue("delta"))) == null)
    expect(
        "listKeys no longer answers with the deleted key",
        keys(kv.listKeys(ListKeys("", 0uL))),
        listOf("alpha", AWKWARD, "beta"),
    )
}

fun main() =
    runBlocking {
        // A hung answer would otherwise wait for the test's own timeout, with
        // nothing to say for itself.
        Thread {
            Thread.sleep(60_000)
            System.err.println("timed out: an operation never answered")
            exitProcess(2)
        }.apply { isDaemon = true }.start()

        store = "InMemoryKeyValueHandler"
        val memory = InMemoryKeyValueHandler()
        check(memory)

        // The store keeps a copy of what it is given and gives out a copy of
        // what it holds, so a caller that changes a list does not change the
        // stored value.
        val given = mutableListOf<UByte>(1u, 2u, 3u)
        memory.set(SetValue("copied", given))
        given[0] = 9u
        expect(
            "a change to the list given to set does not change the stored value",
            bytes(memory.get(GetValue("copied"))),
            listOf<UByte>(1u, 2u, 3u),
        )
        // If the list in the answer is read-only, the caller cannot change it,
        // and the check passes.
        val answered = bytes(memory.get(GetValue("copied")))
        if (answered is MutableList<UByte>) {
            try {
                answered[0] = 9u
            } catch (_: UnsupportedOperationException) {
            }
        }
        expect(
            "a change to the list in the answer to get does not change the stored value",
            bytes(memory.get(GetValue("copied"))),
            listOf<UByte>(1u, 2u, 3u),
        )

        if (failures.isEmpty()) {
            println("HARNESS OK")
        } else {
            failures.forEach { println("FAILED: $it") }
            exitProcess(1)
        }
    }
