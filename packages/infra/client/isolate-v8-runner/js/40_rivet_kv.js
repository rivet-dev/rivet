import {
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
	op_rivet_kv_put,
	op_rivet_kv_put_batch,
	op_rivet_kv_delete,
} from "ext:core/ops";
import { core } from "ext:core/mod.js";

/**
 * Retrieves a value from the key-value store.
 *
 * @param {string} key - The key to retrieve the value for.
 * @param {Object} [options] - Optional settings.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the data.
 *                                                      If "arrayBuffer", returns an ArrayBuffer.
 *                                                      Otherwise, returns the deserialized value.
 * @returns {Promise<any|undefined>} The retrieved value, or undefined if the key does not exist.
 */
async function get(key, options) {
	let value = (await op_rivet_kv_get(key)) ?? undefined;

	return deserializeValue(key, value, options);
}

/**
 * Asynchronously retrieves a batch of key-value pairs.
 *
 * @param {string[]} keys - A list of keys to retrieve.
 * @param {Object} [options] - Optional settings.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the data.
 *                                                      If "arrayBuffer", returns an ArrayBuffer.
 *                                                      Otherwise, returns the deserialized value.
 * @returns {Promise<Map<string, any>>} The retrieved values. Keys that have no value in the key-value store
 * 										will not be present.
 */
async function getBatch(keys, options) {
	let values = await op_rivet_kv_get_batch(keys, options);

	let deserializedValues = new Map();

	for (let key in values) {
		deserializedValues.set(key, deserializeValue(key, values[key], options));
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
	await op_rivet_kv_put(key, core.serialize(value));
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
		serializedObj.set(key, core.serialize(obj[key]));
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

function deserializeValue(key, data, options) {
	if (data != undefined) {
		let format = options?.format ?? "value";

		if (format == "value") {
			try {
				return core.deserialize(data, { forStorage: true });
			} catch (e) {
				throw new Error(
					`Could not deserialize data in key "${key}". You must use options.format = "arrayBuffer".`,
					{ cause: e }
				);
			}
		} else if (format == "arrayBuffer") {
			return data.buffer;
		} else {
			throw Error(`Invalid format: "${options.format}". Expected "value" or "arrayBuffer".`);
		}
	}

	return undefined;
}

export { get, getBatch, put, putBatch, delete_, deleteBatch };
