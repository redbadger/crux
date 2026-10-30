using System.Collections.ObjectModel;
using System.Threading.Tasks;

public interface IStoreHandler
{
    Task<StoreValue> Get(Get operation);
}

public sealed class InMemoryStoreHandler : IStoreHandler
{
    public Task<StoreValue> Get(Get operation) =>
        Task.FromResult(new StoreValue { Value = new ObservableCollection<byte>() });
}
