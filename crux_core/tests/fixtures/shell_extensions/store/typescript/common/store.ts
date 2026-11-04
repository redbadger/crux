export interface StoreHandler {
  get(operation: Get): Promise<StoreValue>;
}

export const inMemoryStoreHandler: StoreHandler = {
  async get(operation: Get): Promise<StoreValue> {
    return new StoreValue([operation.key.length]);
  },
};
