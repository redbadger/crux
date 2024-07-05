using System;
using System.Collections.Generic;

namespace SharedTypes
{
    public static class Requests
    {
        public static List<Request> BincodeDeserialize(ArraySegment<byte> input)
        {
            Serde.IDeserializer deserializer = new Bincode.BincodeDeserializer(input);
            deserializer.increase_container_depth();

            var length = deserializer.deserialize_len();
            var requests = new List<Request>();
            for (var i = 0; i < length; ++i)
            {
                while (deserializer.get_buffer_offset() < input.Count)
                {
                    var req = Request.Deserialize(deserializer);
                    requests.Add(req);
                }
            }
            
            deserializer.decrease_container_depth();
            return requests;
        }
    }
}
