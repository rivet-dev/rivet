// DO NOT MODIFY
//
// Generated from sdks/actor/bridge/

/**
 * Options for the `get` function.
 */
export interface GetOptions {
	format?: "value" | "arrayBuffer";
}
/**
 * Retrieves a value from the key-value store.
 */
export declare function get<K, V>(
	key: K,
	options?: GetOptions,
): Promise<V | null>;
/**
 * Options for the `getBatch` function.
 */
export interface GetBatchOptions {
	format?: "value" | "arrayBuffer";
}
/**
 * Retrieves a batch of key-value pairs.
 */
export declare function getBatch<K extends Array<unknown>, V>(
	keys: K,
	options?: GetBatchOptions,
): Promise<HashMap<K[number], V>>;
/**
 * Options for the `list` function.
 */
export interface ListOptions<K> {
	format?: "value" | "arrayBuffer";
	start?: K;
	startAfter?: K;
	end?: K;
	prefix?: K;
	reverse?: boolean;
	limit?: number;
}
/**
 * Retrieves all key-value pairs in the KV store. When using any of the options, the keys lexicographic order
 * is used for filtering.
 *
 * @param {ListOptions} [options] - Options.
 * @returns {Promise<HashMap<Key, Entry>>} The retrieved values.
 */
export declare function list<K, V>(
	options?: ListOptions<K>,
): Promise<HashMap<K, V>>;
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
export declare function put<K, V>(
	key: K,
	value: V | ArrayBuffer,
	options?: PutOptions,
): Promise<void>;
/**
 * Options for the `putBatch` function.
 */
export interface PutBatchOptions {
	format?: "value" | "arrayBuffer";
}
/**
 * Stores a batch of key-value pairs.
 *
 * @param {Map<Key, Entry | ArrayBuffer>} obj - An object containing key-value pairs to be stored.
 * @param {PutBatchOptions} [options] - Options.
 * @returns {Promise<void>} A promise that resolves when the batch operation is complete.
 */
export declare function putBatch<K, V>(
	obj: Map<K, V | ArrayBuffer>,
	options?: PutBatchOptions,
): Promise<void>;
/**
 * Deletes a key-value pair from the key-value store.
 *
 * @param {Key} key - The key of the key-value pair to delete.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
export declare function delete_<K>(key: K): Promise<void>;
/**
 * Deletes a batch of key-value pairs from the key-value store.
 *
 * @param {Key[]} keys - A list of keys to delete.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
export declare function deleteBatch<K extends Array<unknown>>(
	keys: K,
): Promise<void>;
/**
 * Deletes all data from the key-value store. **This CANNOT be undone.**
 *
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
export declare function deleteAll(): Promise<void>;
declare class HashMap<K, V> {
	constructor(internal: [K, V][]);
	get(key: K): V | undefined;
	/**
	 * Returns a map of keys to values. **WARNING** Using `.get` on the returned map does not work as expected
	 * with complex types (arrays, objects, etc). Use `.get` on this class instead.
	 */
	raw(): Map<K, V>;
	array(): [K, V][];
	entries(): ArrayIterator<[K, V]>;
	[Symbol.iterator](): ArrayIterator<[K, V]>;
}
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
export type Kv = typeof KV_NAMESPACE;
