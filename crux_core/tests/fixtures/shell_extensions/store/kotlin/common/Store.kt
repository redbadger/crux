import java.util.Locale

interface StoreHandler {
    suspend fun get(operation: Get): StoreValue
}

object InMemoryStoreHandler : StoreHandler {
    override suspend fun get(operation: Get): StoreValue =
        StoreValue(operation.key.lowercase(Locale.ROOT).map { it.code.toUByte() })
}
