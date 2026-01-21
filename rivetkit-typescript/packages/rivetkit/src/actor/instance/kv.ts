import type { ActorDriver } from "../driver";
import { makePrefixedKey, removePrefixFromKey } from "./keys";

/**
 * User-facing KV storage interface exposed on ActorContext.
 */
type KvValueType = "text" | "arrayBuffer" | "binary";
type KvKeyType = "text" | "binary";
type KvKey = Uint8Array | string;

type KvValueTypeMap = {
	text: string;
	arrayBuffer: ArrayBuffer;
	binary: Uint8Array;
};

type KvKeyTypeMap = {
	text: string;
	binary: Uint8Array;
};

type KvValueOptions<T extends KvValueType = "text"> = {
	type?: T;
};

type KvListOptions<
	T extends KvValueType = "text",
	K extends KvKeyType = "text",
> = KvValueOptions<T> & {
	keyType?: K;
};

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

function encodeKey<K extends KvKeyType = KvKeyType>(
	key: KvKeyTypeMap[K],
	keyType?: K,
): Uint8Array {
	if (key instanceof Uint8Array) {
		return key;
	}
	const resolvedKeyType = keyType ?? "text";
	if (resolvedKeyType === "binary") {
		throw new TypeError("Expected a Uint8Array when keyType is binary");
	}
	return textEncoder.encode(key);
}

function decodeKey<K extends KvKeyType = "text">(
	key: Uint8Array,
	keyType?: K,
): KvKeyTypeMap[K] {
	const resolvedKeyType = keyType ?? "text";
	switch (resolvedKeyType) {
		case "text":
			return textDecoder.decode(key) as KvKeyTypeMap[K];
		case "binary":
			return key as KvKeyTypeMap[K];
		default:
			throw new TypeError("Invalid kv key type");
	}
}

function resolveValueType(
	value: string | Uint8Array | ArrayBuffer,
): KvValueType {
	if (typeof value === "string") {
		return "text";
	}
	if (value instanceof Uint8Array) {
		return "binary";
	}
	if (value instanceof ArrayBuffer) {
		return "arrayBuffer";
	}
	throw new TypeError("Invalid kv value");
}

function encodeValue<T extends KvValueType = KvValueType>(
	value: KvValueTypeMap[T],
	options?: KvValueOptions<T>,
): Uint8Array {
	const type =
		options?.type ??
		resolveValueType(value as string | Uint8Array | ArrayBuffer);
	switch (type) {
		case "text":
			if (typeof value !== "string") {
				throw new TypeError("Expected a string when type is text");
			}
			return textEncoder.encode(value);
		case "arrayBuffer":
			if (!(value instanceof ArrayBuffer)) {
				throw new TypeError("Expected an ArrayBuffer when type is arrayBuffer");
			}
			return new Uint8Array(value);
		case "binary":
			if (!(value instanceof Uint8Array)) {
				throw new TypeError("Expected a Uint8Array when type is binary");
			}
			return value;
		default:
			throw new TypeError("Invalid kv value type");
	}
}

function decodeValue<T extends KvValueType = "text">(
	value: Uint8Array,
	options?: KvValueOptions<T>,
): KvValueTypeMap[T] {
	const type = options?.type ?? "text";
	switch (type) {
		case "text":
			return textDecoder.decode(value) as KvValueTypeMap[T];
		case "arrayBuffer": {
			const copy = new Uint8Array(value.byteLength);
			copy.set(value);
			return copy.buffer as KvValueTypeMap[T];
		}
		case "binary":
			return value as KvValueTypeMap[T];
		default:
			throw new TypeError("Invalid kv value type");
	}
}

export class ActorKv {
	#driver: ActorDriver;
	#actorId: string;

	constructor(driver: ActorDriver, actorId: string) {
		this.#driver = driver;
		this.#actorId = actorId;
	}

	/**
	 * Get a single value by key.
	 */
	async get<T extends KvValueType = "text">(
		key: KvKey,
		options?: KvValueOptions<T>,
	): Promise<KvValueTypeMap[T] | null> {
		const results = await this.#driver.kvBatchGet(this.#actorId, [
			makePrefixedKey(encodeKey(key)),
		]);
		const result = results[0];
		if (!result) {
			return null;
		}
		return decodeValue(result, options);
	}

	/**
	 * Get multiple values by keys.
	 */
	async getBatch<T extends KvValueType = "text">(
		keys: KvKey[],
		options?: KvValueOptions<T>,
	): Promise<(KvValueTypeMap[T] | null)[]> {
		const prefixedKeys = keys.map((key) =>
			makePrefixedKey(encodeKey(key)),
		);
		const results = await this.#driver.kvBatchGet(
			this.#actorId,
			prefixedKeys,
		);
		return results.map((result) =>
			result ? decodeValue(result, options) : null,
		);
	}

	/**
	 * Put a single key-value pair.
	 */
	async put<T extends KvValueType = KvValueType>(
		key: KvKey,
		value: KvValueTypeMap[T],
		options?: KvValueOptions<T>,
	): Promise<void> {
		await this.#driver.kvBatchPut(this.#actorId, [
			[makePrefixedKey(encodeKey(key)), encodeValue(value, options)],
		]);
	}

	/**
	 * Put multiple key-value pairs.
	 */
	async putBatch<T extends KvValueType = KvValueType>(
		entries: [KvKey, KvValueTypeMap[T]][],
		options?: KvValueOptions<T>,
	): Promise<void> {
		const prefixedEntries: [Uint8Array, Uint8Array][] = entries.map(
			([key, value]) => [
				makePrefixedKey(encodeKey(key)),
				encodeValue(value, options),
			],
		);
		await this.#driver.kvBatchPut(this.#actorId, prefixedEntries);
	}

	/**
	 * Delete a single key.
	 */
	async delete(key: KvKey): Promise<void> {
		await this.#driver.kvBatchDelete(this.#actorId, [
			makePrefixedKey(encodeKey(key)),
		]);
	}

	/**
	 * Delete multiple keys.
	 */
	async deleteBatch(keys: KvKey[]): Promise<void> {
		const prefixedKeys = keys.map((key) =>
			makePrefixedKey(encodeKey(key)),
		);
		await this.#driver.kvBatchDelete(this.#actorId, prefixedKeys);
	}

	/**
	 * List all keys with a given prefix.
	 * Returns key-value pairs where keys have the user prefix removed.
	 */
	async list<T extends KvValueType = "text", K extends KvKeyType = "text">(
		prefix: KvKeyTypeMap[K],
		options?: KvListOptions<T, K>,
	): Promise<[KvKeyTypeMap[K], KvValueTypeMap[T]][]> {
		const prefixedPrefix = makePrefixedKey(
			encodeKey(prefix, options?.keyType),
		);
		const results = await this.#driver.kvListPrefix(
			this.#actorId,
			prefixedPrefix,
		);
		return results.map(([key, value]) => [
			decodeKey<K>(removePrefixFromKey(key), options?.keyType),
			decodeValue<T>(value, options),
		]);
	}
}
