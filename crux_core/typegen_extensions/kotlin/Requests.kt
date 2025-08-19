object Requests {

    @JvmStatic
    @Throws(com.novi.serde.DeserializationError::class)
    fun bincodeDeserialize(input: ByteArray?): List<Request> {
        if (input == null) {
            throw com.novi.serde.DeserializationError("Cannot deserialize null array")
        }

        val deserializer: com.novi.serde.Deserializer = com.novi.bincode.BincodeDeserializer(input)
        deserializer.increase_container_depth()

        val length = deserializer.deserialize_len()

        val value = mutableListOf<Request>()

        for (i in 0 until length) {
            value.add(Request.deserialize(deserializer))
        }

        deserializer.decrease_container_depth()

        if (deserializer.get_buffer_offset() < input.size) {
            throw com.novi.serde.DeserializationError("Some input bytes were not read")
        }
        return value
    }
}
