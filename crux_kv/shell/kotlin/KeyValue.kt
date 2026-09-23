import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException
import java.net.URLDecoder
import java.net.URLEncoder

/// The shell side of `crux_kv`, as one method per operation.
///
/// Each method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```kotlin
/// class MyHandler(context: Context) : EffectHandler {
///     private val kv = FileKeyValueHandler(context.filesDir)
///
///     override suspend fun kvGet(operation: GetValue): ValueResult = kv.get(operation)
///     override suspend fun kvSet(operation: SetValue): ValueResult = kv.set(operation)
/// }
/// ```
///
/// Implement it yourself to put the store somewhere else — `DataStore`, a
/// database, a keystore — or to replace a single method and leave the rest.
///
/// ## What each operation answers with
///
/// `get` answers with what is stored under the key, `Value.None` when nothing
/// is. `set` and `delete` answer with the value they replaced or removed, which
/// is also `Value.None` when there was none. Nothing here is an error unless
/// the store itself failed; a missing key is an answer.
interface KeyValueHandler {
    /// Read the bytes stored under `operation.key`.
    suspend fun get(operation: GetValue): ValueResult

    /// Write `operation.value`, answering with the value it replaced.
    suspend fun set(operation: SetValue): ValueResult

    /// Remove `operation.key`, answering with the value it removed.
    suspend fun delete(operation: DeleteValue): ValueResult

    /// Whether `operation.key` is in the store.
    suspend fun exists(operation: KeyExists): ExistsResult

    /// The keys starting with `operation.prefix`, from `operation.cursor`.
    suspend fun listKeys(operation: ListKeys): KeysResult
}

/// A [KeyValueHandler] that keeps one file per key in a directory.
///
/// The JDK has no key-value store of its own, and files are what every JVM
/// has: no database, no `android.*`, and a store an app can point at
/// `context.filesDir`, a temporary directory in a test, or anywhere else it can
/// write.
///
/// A key is percent-encoded to make its file name, so a key with a `/` in it is
/// one file and not a directory, and `listKeys` can read the keys back out of
/// the directory listing.
///
/// ## What it gives you, and what it does not
///
/// A write is atomic and forced to the device, so a value survives a crash
/// whole or not at all. Beyond that this is a plain file store: every read
/// goes to the filesystem, there is no cache, nothing spans more than one key,
/// and nothing observes a change. A shell that wants those — `DataStore` or
/// Room on Android, a database anywhere — conforms its own type to
/// [KeyValueHandler] and provides that instead, which is a change to one
/// declaration.
///
/// @param directory where the files live; created on first use.
class FileKeyValueHandler(
    private val directory: File,
) : KeyValueHandler {
    override suspend fun get(operation: GetValue): ValueResult =
        withContext(Dispatchers.IO) {
            io { ValueResult.Ok(read(operation.key)) }
        }

    override suspend fun set(operation: SetValue): ValueResult =
        withContext(Dispatchers.IO) {
            io {
                val previous = read(operation.key)
                directory.mkdirs()
                write(file(operation.key), operation.value.map { it.toByte() }.toByteArray())
                ValueResult.Ok(previous)
            }
        }

    override suspend fun delete(operation: DeleteValue): ValueResult =
        withContext(Dispatchers.IO) {
            io {
                val previous = read(operation.key)
                file(operation.key).delete()
                ValueResult.Ok(previous)
            }
        }

    override suspend fun exists(operation: KeyExists): ExistsResult =
        withContext(Dispatchers.IO) {
            try {
                ExistsResult.Ok(file(operation.key).isFile)
            } catch (e: IOException) {
                ExistsResult.Err(KeyValueError.Io(e.message ?: "IO error"))
            } catch (e: SecurityException) {
                ExistsResult.Err(KeyValueError.Io(e.message ?: "access denied"))
            }
        }

    /// One page holds every remaining key, so the answer's cursor is always 0:
    /// the directory is listed in one go anyway, and paging it would only
    /// invite a caller to hold a cursor across a write.
    override suspend fun listKeys(operation: ListKeys): KeysResult =
        withContext(Dispatchers.IO) {
            val keys =
                try {
                    (directory.listFiles() ?: emptyArray())
                        .filter { it.isFile }
                        .map { URLDecoder.decode(it.name, Charsets.UTF_8.name()) }
                        .filter { it.startsWith(operation.prefix) }
                        .sorted()
                } catch (e: IOException) {
                    return@withContext KeysResult.Err(KeyValueError.Io(e.message ?: "IO error"))
                } catch (e: SecurityException) {
                    return@withContext KeysResult.Err(KeyValueError.Io(e.message ?: "access denied"))
                }

            if (operation.cursor > keys.size.toULong()) {
                return@withContext KeysResult.Err(KeyValueError.CursorNotFound)
            }

            KeysResult.Ok(KeyPage(keys.drop(operation.cursor.toInt()), 0uL))
        }

    private fun read(key: String): Value {
        val file = file(key)
        return if (file.isFile) Value.Bytes(file.readBytes().map { it.toUByte() }) else Value.None
    }

    private fun file(key: String): File = File(directory, URLEncoder.encode(key, Charsets.UTF_8.name()))

    /// Writes `bytes` to `target` so that a reader sees either the whole of
    /// the new value or the whole of the old one.
    ///
    /// The bytes go to a temporary file, are forced to the device, and are
    /// then renamed over the target. A rename within one filesystem is
    /// atomic, so a crash or a pulled battery leaves either the old value or
    /// the new one, never half of either.
    ///
    /// The temporary file is made in a `tmp` directory inside the store, so
    /// the rename stays within one filesystem — which is what `ATOMIC_MOVE`
    /// needs — and so that a temporary left behind by a killed process is
    /// never mistaken for a key: `listKeys` reads the files of the store
    /// directory, and a directory is not one of them.
    ///
    /// A filesystem is still free to refuse an atomic move, so the fallback is
    /// a plain replace — no longer atomic, and a crash during it can lose the
    /// value, but the alternative is failing a write that would otherwise
    /// succeed.
    private fun write(
        target: File,
        bytes: ByteArray,
    ) {
        val temporaries = File(directory, "tmp").apply { mkdirs() }
        val temporary = File.createTempFile("value", ".tmp", temporaries)
        try {
            java.io.FileOutputStream(temporary).use { out ->
                out.write(bytes)
                out.fd.sync()
            }
            try {
                java.nio.file.Files.move(
                    temporary.toPath(),
                    target.toPath(),
                    java.nio.file.StandardCopyOption.ATOMIC_MOVE,
                )
            } catch (_: java.nio.file.AtomicMoveNotSupportedException) {
                java.nio.file.Files.move(
                    temporary.toPath(),
                    target.toPath(),
                    java.nio.file.StandardCopyOption.REPLACE_EXISTING,
                )
            }
        } finally {
            temporary.delete()
        }
    }

    private inline fun io(block: () -> ValueResult): ValueResult =
        try {
            block()
        } catch (e: IOException) {
            ValueResult.Err(KeyValueError.Io(e.message ?: "IO error"))
        } catch (e: SecurityException) {
            ValueResult.Err(KeyValueError.Io(e.message ?: "access denied"))
        }
}
