{
    public sealed class Requests
    {
        public static System.Collections.Generic.List<Request> BincodeDeserialize(byte[] input)
        {
            if (input == null)
            {
                throw new Serde.DeserializationException("Cannot deserialize null array");
            }

            Serde.IDeserializer deserializer = new Bincode.BincodeDeserializer(input);
            deserializer.increase_container_depth();

            long length = deserializer.deserialize_len();

            System.Collections.Generic.List<Request> value = new System.Collections.Generic.List<Request>();

            for (int i = 0; i < length; i++)
            {
                value.Add(Request.Deserialize(deserializer));
            }

            deserializer.decrease_container_depth();

            if (deserializer.get_buffer_offset() < input.Length)
            {
                throw new Serde.DeserializationException("Some input bytes were not read");
            }

            return value;
        }
    }
}
