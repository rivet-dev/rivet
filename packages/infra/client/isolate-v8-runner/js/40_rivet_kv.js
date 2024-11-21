import {
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
	op_rivet_kv_list,
	op_rivet_kv_put,
	op_rivet_kv_put_batch,
	op_rivet_kv_delete,
	op_rivet_kv_delete_batch,
} from "ext:core/ops";
import { core } from "ext:core/mod.js";

/**
 * Retrieves a value from the key-value store.
 *
 * @param {string} key - The key to retrieve the value for.
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the data.
 *                                                      If "arrayBuffer", returns an ArrayBuffer.
 *                                                      Otherwise, returns the deserialized value.
 * @returns {Promise<any|undefined>} The retrieved value, or undefined if the key does not exist.
 */
async function get(key, options) {
	let entry = (await op_rivet_kv_get(key)) ?? undefined;

	return deserializeValue(key, entry.value, options?.format);
}

/**
 * Retrieves a batch of key-value pairs.
 *
 * @param {string[]} keys - A list of keys to retrieve.
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the data.
 *                                                      If "arrayBuffer", returns an ArrayBuffer.
 *                                                      Otherwise, returns the deserialized value.
 * @returns {Promise<Map<string, any>>} The retrieved values. Keys that have no value in the key-value store
 * 										will not be present.
 */
async function getBatch(keys, options) {
	let entries = await op_rivet_kv_get_batch(keys);

	let deserializedValues = new Map();

	for (let key in entries) {
		deserializedValues.set(key, deserializeValue(key, entries[key].value, options?.format));
	}

	return deserializedValues;
}

/**
 * Retrieves all key-value pairs in the KV store. When using any of the options, the keys lexicographic order
 * is used for filtering.
 *
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the data.
 *                                                      If "arrayBuffer", returns an ArrayBuffer.
 *                                                      Otherwise, returns the deserialized value.
 * @param {string} [options.start] - The key to start listing results from (inclusive).
 * @param {string} [options.startAfter] - The key to start listing results after (exclusive). Cannot be used
 * with start.
 * @param {string} [options.end] - The key to end listing results at (exclusive).
 * @param {string} [options.prefix] - Restricts results to keys that start with the given prefix.
 * @param {boolean} [options.reverse] - If true, results are returned in descending order. Start still defines
 * the smallest key and end still defines the largest key in lexicographic order.
 * @param {number} [options.limit] - The maximum number of key-value pairs to return.
 * @returns {Promise<Map<string, any>>} The retrieved values. Keys that have no value in the key-value store
 * 										will not be present.
 */
async function list(options) {
	let query;
	if (options.prefix) {
		query = {
			prefix: options.prefix,
		};
	} else if (options.start) {
		if (!options.end) {
			throw new Error("must set options.end with options.start");
		}

		query = {
			rangeInclusive: [options.start, options.end],
		};
	} else if (options.startAfter) {
		if (!options.end) {
			throw new Error("must set options.end with options.startAfter");
		}

		query = {
			rangeExclusive: [options.startAfter, options.end],
		};
	} else if (options.end) {
		throw new Error("must set options.start or options.startAfter with options.end");
	} else {
		query = { all: {} };
	}

	let entries = await op_rivet_kv_list(query, options?.reverse ?? false, options?.limit);

	let deserializedValues = new Map();

	for (let key in entries) {
		deserializedValues.set(key, deserializeValue(key, entries[key].value, options?.format));
	}

	return deserializedValues;
}

/**
 * Stores a key-value pair in the key-value store.
 *
 * @param {string} key - The key under which the value will be stored.
 * @param {any} value - The value to be stored, which will be serialized.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
async function put(key, value) {
	validateType(value);

	await op_rivet_kv_put(key, core.serialize(value, { forStorage: true }));
}

/**
 * Asynchronously stores a batch of key-value pairs.
 *
 * @param {Object} obj - An object containing key-value pairs to be stored.
 * @returns {Promise<void>} A promise that resolves when the batch operation is complete.
 */
async function putBatch(obj) {
	let serializedObj = new Map();

	for (let key in obj) {
		validateType(obj[key], key);
		serializedObj.set(key, core.serialize(obj[key], { forStorage: true }));
	}

	await op_rivet_kv_put_batch(serializedObj);
}

/**
 * Deletes a key-value pair from the key-value store.
 *
 * @param {string} key - The key of the key-value pair to delete.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
async function delete_(key) {
	return await op_rivet_kv_delete(key);
}

async function deleteBatch(keys) {
	return await op_rivet_kv_delete_batch(keys);
}

// See https://github.com/denoland/deno/issues/12067#issuecomment-1975001079
function validateType(value, key) {
	let keyText = key ? ` in key "{key}"` : "";

	if (value instanceof Blob) {
		throw new Error(
			`The type ${value.constructor.name}${keyText} is not serializable in Deno, but you can use a TypedArray instead. See https://github.com/denoland/deno/issues/12067#issuecomment-1975001079.`
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
			`The type ${value.constructor.name}${keyText} is not serializable in Deno. See https://github.com/denoland/deno/issues/12067#issuecomment-1975001079.`
		);
	}
}

function deserializeValue(key, data, format = "value") {
	if (data != undefined) {
		if (format == "value") {
			try {
				return core.deserialize(data, { forStorage: true });
			} catch (e) {
				throw new Error(
					`could not deserialize data in key "${key}". you must use options.format = "arrayBuffer".`,
					{ cause: e }
				);
			}
		} else if (format == "arrayBuffer") {
			return data.buffer;
		} else {
			throw Error(`invalid format: "${options.format}". expected "value" or "arrayBuffer".`);
		}
	}

	return undefined;
}

export { get, getBatch, list, put, putBatch, delete_, deleteBatch };
