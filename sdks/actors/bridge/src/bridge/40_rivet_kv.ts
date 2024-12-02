import {
	op_rivet_kv_delete,
	op_rivet_kv_delete_all,
	op_rivet_kv_delete_batch,
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
	op_rivet_kv_list,
	op_rivet_kv_put,
	op_rivet_kv_put_batch,
} from "ext:core/ops";
import type { InKey, OutKey, ListQuery } from "internal_types";
import { core } from "ext:core/mod.js";

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
async function get(key: Key, options?: GetOptions): Promise<Entry | null> {
	let entry = await op_rivet_kv_get(serializeKey(key));
	if (entry == null) return null;

	return deserializeValue(key, entry.value, options?.format);
}

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
async function getBatch(keys: Key[], options?: GetBatchOptions): Promise<Map<Key, Entry>> {
	let entries = await op_rivet_kv_get_batch(keys.map((x) => serializeKey(x)));

	let deserializedValues = new Map<Key, Entry>();

	for (let [key, entry] of entries) {
		let jsKey = deserializeKey(key);
		deserializedValues.set(jsKey, deserializeValue(jsKey, entry.value, options?.format));
	}

	return deserializedValues;
}

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
async function list(options?: ListOptions): Promise<Map<Key, Entry>> {
	// Build query
	let query: ListQuery;
	if (options?.prefix) {
		query = {
			prefix: serializeListKey(options.prefix),
		};
	} else if (options?.start) {
		if (!options.end) {
			throw new Error("must set options.end with options.start");
		}

		query = {
			rangeInclusive: [serializeListKey(options.start), serializeKey(options.end)],
		};
	} else if (options?.startAfter) {
		if (!options.end) {
			throw new Error("must set options.end with options.startAfter");
		}

		query = {
			rangeExclusive: [serializeListKey(options.startAfter), serializeKey(options.end)],
		};
	} else if (options?.end) {
		throw new Error("must set options.start or options.startAfter with options.end");
	} else {
		query = { all: {} };
	}

	let entries = await op_rivet_kv_list(query, options?.reverse ?? false, options?.limit);
	let deserializedValues = new Map<Key, Entry>();

	for (let [key, entry] of entries) {
		let jsKey = deserializeKey(key);
		deserializedValues.set(jsKey, deserializeValue(jsKey, entry.value, options?.format));
	}

	return deserializedValues;
}

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
async function put(key: Key, value: Entry | ArrayBuffer, options?: PutOptions): Promise<void> {
	validateType(value, null, options?.format);
	let format = options?.format ?? "value";

	let serializedValue;
	if (format == "value") {
		serializedValue = core.serialize(value, { forStorage: true });
	} else if (format == "arrayBuffer") {
		if (value instanceof ArrayBuffer) serializedValue = new Uint8Array(value);
		else throw new Error(`value must be of type \`ArrayBuffer\` if format is "arrayBuffer"`);
	}

	await op_rivet_kv_put(serializeKey(key), serializedValue!);
}

/**
 * Options for the `putBatch` function.
 */
export interface PutBatchOptions {
	format?: "value" | "arrayBuffer";
}

/**
 * Asynchronously stores a batch of key-value pairs.
 *
 * @param {Map<Key, Entry | ArrayBuffer>} obj - An object containing key-value pairs to be stored.
 * @param {PutBatchOptions} [options] - Options.
 * @returns {Promise<void>} A promise that resolves when the batch operation is complete.
 */
async function putBatch(obj: Map<Key, Entry | ArrayBuffer>, options?: PutBatchOptions): Promise<void> {
	let serializedObj = new Map<InKey, Uint8Array>();
	let format = options?.format ?? "value";

	for (let [key, value] of obj) {
		validateType(value, key, format);

		let serializedValue;
		if (format == "value") {
			serializedValue = core.serialize(value, { forStorage: true });
		} else if (format == "arrayBuffer") {
			if (value instanceof ArrayBuffer) serializedValue = new Uint8Array(value);
			else
				throw new Error(
					`value in key "${key}" must be of type \`ArrayBuffer\` if format is "arrayBuffer"`
				);
		}

		serializedObj.set(serializeKey(key), serializedValue!);
	}

	await op_rivet_kv_put_batch(serializedObj);
}

/**
 * Deletes a key-value pair from the key-value store.
 *
 * @param {Key} key - The key of the key-value pair to delete.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
async function delete_(key: Key): Promise<void> {
	return await op_rivet_kv_delete(serializeKey(key));
}

async function deleteBatch(keys: Key[]): Promise<void> {
	return await op_rivet_kv_delete_batch(keys.map((x) => serializeKey(x)));
}

/**
 * Deletes all data from the key-value store. **This CANNOT be undone.**
 *
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
async function deleteAll(): Promise<void> {
	return await op_rivet_kv_delete_all();
}

function validateType(
	value: Entry | ArrayBuffer,
	key: Key | null,
	format: "value" | "arrayBuffer" = "value"
): void {
	let keyText = key ? ` in key "{key}"` : "";

	if (format == "value") {
		if (value instanceof Blob) {
			throw new Error(
				`the type ${value.constructor.name}${keyText} is not serializable in Deno, but you can use a TypedArray instead. See https://github.com/denoland/deno/issues/12067#issuecomment-1975001079.`
			);
		}
		if (
			value instanceof CryptoKey ||
			value instanceof DOMException ||
			// Not defined in Deno
			// value instanceof RTCCertificate ||
			// We don't load in the canvas ext into the the Deno runtime for Rivet
			// value instanceof ImageBitmap ||
			value instanceof ImageData
		) {
			throw new Error(
				`the type ${value.constructor.name}${keyText} is not serializable in Deno. See https://github.com/denoland/deno/issues/12067#issuecomment-1975001079.`
			);
		}
	} else if (format == "arrayBuffer") {
		if (!(value instanceof ArrayBuffer)) {
			throw new Error(`value must be an ArrayBuffer if options.format = "arrayBuffer".`);
		}
	} else {
		throw new Error("unexpected key type from KV driver");
	}
}

function serializeKey(key: Key): InKey {
	if (key instanceof Array) {
		return { jsInKey: key.map((x) => core.serialize(x)) };
	} else {
		return { jsInKey: [core.serialize(key)] };
	}
}

function serializeListKey(key: Key): Uint8Array[] {
	if (key instanceof Array) {
		return key.map((x) => core.serialize(x));
	} else {
		return [core.serialize(key)];
	}
}

function deserializeKey(key: OutKey): Key {
	if ("inKey" in key || "outKey" in key) {
		let jsKey = key.inKey ?? key.outKey;

		let tuple = jsKey.map((x) => core.deserialize(x));

		if (tuple.length == 1) return tuple[0];
		else return tuple;
	} else {
		throw new Error("unexpected key type from KV driver");
	}
}

function deserializeValue(key: Key, value: Uint8Array, format: "value" | "arrayBuffer" = "value"): Entry {
	if (value == undefined) return value;

	if (format == "value") {
		try {
			return core.deserialize(value, { forStorage: true });
		} catch (e) {
			throw new Error(
				`could not deserialize value in key "${key}". you must use options.format = "arrayBuffer".`,
				{ cause: e }
			);
		}
	} else if (format == "arrayBuffer") {
		return value.buffer;
	} else {
		throw Error(`invalid format: "${format}". expected "value" or "arrayBuffer".`);
	}
}

export const KV_NAMESPACE = {
	get,
	getBatch,
	list,
	put,
	putBatch,
	delete: delete_,
	deleteBatch,
	deleteAll,
};
