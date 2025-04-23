using System.Diagnostics;
using SharedTypes;
using uniffi.shared;

namespace SimpleCounter
{
    internal class Core
    {
        public ViewModel View { get; private set; }

        public Core()
        {
            // Initialize the ViewModel
            View = ViewModel.BincodeDeserialize(SharedMethods.View());
        }

        public void Update(Event @event)
        {
            // Process the event and get effects
            var effects = SharedMethods.ProcessEvent(@event.BincodeSerialize());

            // Deserialize the effects into requests
            var requests = Requests.BincodeDeserialize(effects);

            // Handle each request 
            foreach (var request in requests)
            {
                ProcessEffect(request);
            }
        }

        private void ProcessEffect(Request request)
        {
            switch (request.effect)
            {
                case Effect.Render:
                    // Update the ViewModel
                    View = ViewModel.BincodeDeserialize(SharedMethods.View());
                    break;

                // Handle other effects here (e.g., HTTP requests, logging, etc.)
                default:
                    Debug.WriteLine($"Unhandled effect: {request.effect}");
                    break;
            }
        }
    }
}
