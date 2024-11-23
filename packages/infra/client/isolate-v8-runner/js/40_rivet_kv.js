import {
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
	op_rivet_kv_list,
	op_rivet_kv_put,
	op_rivet_kv_put_batch,
	op_rivet_kv_delete,
	op_rivet_kv_delete_batch,
	op_rivet_kv_delete_all,
} from "ext:core/ops";
import { core } from "ext:core/mod.js";

/**
 * Retrieves a value from the key-value store.
 *
 * @param {any|any[]} key - The key to retrieve the value for.
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the value.
 * If "arrayBuffer", returns an ArrayBuffer.
 * Otherwise, returns the deserialized value.
 * @returns {Promise<any|undefined>} The retrieved value, or undefined if the key does not exist.
 */
async function get(key, options) {
	let entry = (await op_rivet_kv_get(serializeKey(key))) ?? undefined;

	return deserializeValue(key, entry.value, options?.format);
}

/**
 * Retrieves a batch of key-value pairs.
 *
 * @param {string[]} keys - A list of keys to retrieve.
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the value.
 * If "arrayBuffer", returns an ArrayBuffer.
 * Otherwise, returns the deserialized value.
 * @returns {Promise<Map<string, any>>} The retrieved values. Keys that have no value in the key-value store
 * will not be present.
 */
async function getBatch(keys, options) {
	let entries = await op_rivet_kv_get_batch(keys.map((x) => serializeKey(x)));

	let deserializedValues = new Map();

	for (let [key, entry] of entries) {
		let jsKey = deserializeKey(key);
		deserializedValues.set(jsKey, deserializeValue(jsKey, entry.value, options?.format));
	}

	return deserializedValues;
}

/**
 * Retrieves all key-value pairs in the KV store. When using any of the options, the keys lexicographic order
 * is used for filtering.
 *
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to return the value.
 * If "arrayBuffer", returns an ArrayBuffer.
 * Otherwise, returns the deserialized value.
 * @param {string} [options.start] - The key to start listing results from (inclusive). Cannot be used with
 * startAfter or prefix.
 * @param {string} [options.startAfter] - The key to start listing results after (exclusive). Cannot be used
 * with start or prefix.
 * @param {string} [options.end] - The key to end listing results at (exclusive).
 * @param {string} [options.prefix] - Restricts results to keys that start with the given prefix. Cannot be
 * used with start or startAfter.
 * @param {boolean} [options.reverse] - If true, results are returned in descending order.
 * @param {number} [options.limit] - The maximum number of key-value pairs to return.
 * @returns {Promise<Map<string, any>>} The retrieved values.
 */
async function list(options) {
	// Build query
	let query;
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
	let deserializedValues = new Map();

	for (let [key, entry] of entries) {
		let jsKey = deserializeKey(key);
		deserializedValues.set(jsKey, deserializeValue(jsKey, entry.value, options?.format));
	}

	return deserializedValues;
}

/**
 * Stores a key-value pair in the key-value store.
 *
 * @param {any|any[]} key - The key under which the value will be stored.
 * @param {any|ArrayBuffer} value - The value to be stored, which will be serialized.
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to write the value. `value` must
 * be an ArrayBuffer if this is set to arrayBuffer.
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
async function put(key, givenValue, options) {
	validateType(givenValue, null, options?.format);
	let format = options?.format ?? "value";

	let value;
	if (format == "value") {
		value = core.serialize(givenValue, { forStorage: true });
	} else if (format == "arrayBuffer") {
		value = new Uint8Array(givenValue);
	}

	await op_rivet_kv_put(serializeKey(key), value);
}

/**
 * Asynchronously stores a batch of key-value pairs.
 *
 * @param {Record<any, any|ArrayBuffer>} obj - An object containing key-value pairs to be stored.
 * @param {any|ArrayBuffer} value - The value to be stored, which will be serialized.
 * @param {Object} [options] - Options.
 * @param {('value'|'arrayBuffer')} [options.format] - The format in which to write the values. values in
 * `obj` must be ArrayBuffers if this is set to arrayBuffer.
 * @returns {Promise<void>} A promise that resolves when the batch operation is complete.
 */
async function putBatch(obj, options) {
	let serializedObj = new Map();
	let format = options?.format ?? "value";

	for (let key in obj) {
		let givenValue = obj[key];

		validateType(givenValue, key, format);

		let value;
		if (format == "value") {
			value = core.serialize(givenValue, { forStorage: true });
		} else if (format == "arrayBuffer") {
			value = new Uint8Array(givenValue);
		}

		serializedObj.set(serializeKey(key), value);
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
	return await op_rivet_kv_delete(serializeKey(key));
}

async function deleteBatch(keys) {
	return await op_rivet_kv_delete_batch(keys.map((x) => serializeKey(x)));
}

/**
 * Deletes all data from the key-value store. **This CANNOT be undone.**
 *
 * @returns {Promise<void>} A promise that resolves when the operation is complete.
 */
async function deleteAll() {
	return await op_rivet_kv_delete_all();
}

function validateType(value, key, format = "value") {
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

function serializeKey(key) {
	if (key instanceof Array) {
		return { jsInKey: [key.map((x) => core.serialize(x))] };
	} else {
		return { jsInKey: [core.serialize(key)] };
	}
}

function serializeListKey(key) {
	if (key instanceof Array) {
		return key.map((x) => core.serialize(x));
	} else {
		return [core.serialize(key)];
	}
}

function deserializeKey(key) {
	if ("inKey" in key || "outKey" in key) {
		let jsKey = key.inKey ?? key.outKey;

		let tuple = jsKey.map((x) => core.deserialize(x));

		if (tuple.length == 1) return tuple[0];
		else return tuple;
	} else {
		throw new Error("unexpected key type from KV driver");
	}
}

function deserializeValue(key, value, format = "value") {
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
		throw Error(`invalid format: "${options.format}". expected "value" or "arrayBuffer".`);
	}
}

export { get, getBatch, list, put, putBatch, delete_, deleteBatch, deleteAll };
