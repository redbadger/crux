;

using System.Collections.Generic;
using Facet.Runtime.Bincode;
using Facet.Runtime.Serde;

public static class Requests
{
    public static List<Request> BincodeDeserialize(byte[] input)
    {
        if (input is null || input.Length == 0)
        {
            throw new DeserializationError("Cannot deserialize null or empty input");
        }

        var deserializer = new BincodeDeserializer(input);
        var length = deserializer.DeserializeLen();
        var value = new List<Request>((int)length);
        for (ulong i = 0; i < length; i++)
        {
            value.Add(Request.Deserialize(deserializer));
        }

        if (deserializer.GetBufferOffset() < input.Length)
        {
            throw new DeserializationError("Some input bytes were not read");
        }

        return value;
    }
}
