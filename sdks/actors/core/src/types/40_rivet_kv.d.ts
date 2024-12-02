// DO NOT MODIFY
//
// Generated from sdks/actors-bridge/

export type Key = any;
export type Entry = any;
/**
 * Options for the `get` function.
 */
export interface GetOptions {
    format?: "value" | "arrayBuffer";
}
/**
 * Retrieves a value from the key-value store.
 *
 * @param {Key} key - The key to retrieve the value for.
 * @param {GetOptions} [options] - Options.
 * @returns {Promise<Entry | null>} The retrieved value, or undefined if the key does not exist.
 */
declare function get(key: Key, options?: GetOptions): Promise<Entry | null>;
/**
 * Options for the `getBatch` function.
 */
export interface GetBatchOptions {
    format?: "value" | "arrayBuffer";
}
/**
 * Retrieves a batch of key-value pairs.
 *
 * @param {Key[]} keys - A list of keys to retrieve.
 * @param {GetBatchOptions} [options] - Options.
 * @returns {Promise<Map<Key, Entry>>} The retrieved values.
 */
declare function getBatch(keys: Key[], options?: GetBatchOptions): Promise<Map<Key, Entry>>;
/**
 * Options for the `list` function.
 */
export interface ListOptions {
    format?: "value" | "arrayBuffer";
    start?: Key;
    startAfter?: Key;
    end?: Key;
    prefix?: Key;
    reverse?: boolean;
    limit?: number;
}
/**
 * Retrieves all key-value pairs in the KV store.
 *
 * @param {ListOptions} [options] - Options.
 * @returns {Promise<Map<Key, Entry>>} The retrieved values.
 */
declare function list(options?: ListOptions): Promise<Map<Key, Entry>>;
/**
 * Options for the `put` function.
 */
export interface PutOptions {
    format?: "value" | "arrayBuffer";
}
/**
 * Stores a key-value pair in the key-value store.
 *
 * @param {Key} key - The key under which the value will be stored.
 * @param {Entry | ArrayBuffer} value - The value to be stored, which will be serialized.
 * @param {PutOptions} [options] - Options.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
declare function put(key: Key, value: Entry | ArrayBuffer, options?: PutOptions): Promise<void>;
/**
 * Options for the `putBatch` function.
 */
export interface PutBatchOptions {
    format?: "value" | "arrayBuffer";
}
/**
 * Asynchronously stores a batch of key-value pairs.
 *
 * @param {Record<Key, Entry | ArrayBuffer>} obj - An object containing key-value pairs to be stored.
 * @param {PutBatchOptions} [options] - Options.
 * @returns {Promise<void>} A promise that resolves when the batch operation is complete.
 */
declare function putBatch(obj: Map<Key, Entry | ArrayBuffer>, options?: PutBatchOptions): Promise<void>;
/**
 * Deletes a key-value pair from the key-value store.
 *
 * @param {Key} key - The key of the key-value pair to delete.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
declare function delete_(key: Key): Promise<void>;
declare function deleteBatch(keys: Key[]): Promise<void>;
/**
 * Deletes all data from the key-value store. **This CANNOT be undone.**
 *
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
declare function deleteAll(): Promise<void>;
export declare const KV_NAMESPACE: {
    get: typeof get;
    getBatch: typeof getBatch;
    list: typeof list;
    put: typeof put;
    putBatch: typeof putBatch;
    delete: typeof delete_;
    deleteBatch: typeof deleteBatch;
    deleteAll: typeof deleteAll;
};
export {};
