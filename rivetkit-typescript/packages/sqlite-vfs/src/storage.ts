import { CHUNK_SIZE, getChunkKey, getMetaKey } from "./kv";
import { FILE_META_VERSIONED, CURRENT_VERSION } from "../schemas/file-meta/versioned.js";
import type { FileMeta } from "../schemas/file-meta/mod.js";
import type { KvVfsOptions } from "./types";

function encodeFileMeta(size: number): Uint8Array {
	const meta: FileMeta = { size: BigInt(size) };
	return FILE_META_VERSIONED.serializeWithEmbeddedVersion(
		meta,
		CURRENT_VERSION,
	);
}

function decodeFileMeta(data: Uint8Array): number {
	const meta = FILE_META_VERSIONED.deserializeWithEmbeddedVersion(data);
	return Number(meta.size);
}

export async function loadDatabaseBytes(
	fileName: string,
	options: KvVfsOptions,
	chunkSize: number,
	kvPrefix: number,
): Promise<Uint8Array | null> {
	const metaKey = getMetaKey(fileName, kvPrefix);
	const metaData = await options.get(metaKey);
	if (!metaData) {
		return null;
	}

	const size = decodeFileMeta(metaData);
	if (size <= 0) {
		return new Uint8Array(0);
	}

	const totalChunks = Math.ceil(size / chunkSize);
	const chunkKeys: Uint8Array[] = [];
	for (let chunkIndex = 0; chunkIndex < totalChunks; chunkIndex++) {
		chunkKeys.push(getChunkKey(fileName, chunkIndex, kvPrefix));
	}

	const chunks = await options.getBatch(chunkKeys);
	const buffer = new Uint8Array(size);

	for (let i = 0; i < chunks.length; i++) {
		const chunk = chunks[i];
		const offset = i * chunkSize;
		const remaining = size - offset;
		const length = Math.min(chunkSize, remaining);

		if (chunk) {
			buffer.set(chunk.subarray(0, length), offset);
		} else {
			buffer.fill(0, offset, offset + length);
		}
	}

	return buffer;
}

export async function persistDatabaseBytes(
	fileName: string,
	options: KvVfsOptions,
	chunkSize: number,
	kvPrefix: number,
	bytes: Uint8Array,
): Promise<void> {
	const size = bytes.length;
	const metaKey = getMetaKey(fileName, kvPrefix);
	const existingMeta = await options.get(metaKey);
	const existingSize = existingMeta ? decodeFileMeta(existingMeta) : 0;

	const newChunkCount = Math.ceil(size / chunkSize);
	const oldChunkCount = Math.ceil(existingSize / chunkSize);

	const entries: [Uint8Array, Uint8Array][] = [];
	for (let chunkIndex = 0; chunkIndex < newChunkCount; chunkIndex++) {
		const start = chunkIndex * chunkSize;
		const end = Math.min(start + chunkSize, size);
		const chunk = bytes.subarray(start, end);
		entries.push([getChunkKey(fileName, chunkIndex, kvPrefix), chunk]);
	}

	if (entries.length > 0) {
		await options.putBatch(entries);
	}

	const metaData = encodeFileMeta(size);
	await options.put(metaKey, metaData);

	if (oldChunkCount > newChunkCount) {
		const deleteKeys: Uint8Array[] = [];
		for (let chunkIndex = newChunkCount; chunkIndex < oldChunkCount; chunkIndex++) {
			deleteKeys.push(getChunkKey(fileName, chunkIndex, kvPrefix));
		}
		if (deleteKeys.length > 0) {
			await options.deleteBatch(deleteKeys);
		}
	}
}

export function resolveChunkSize(chunkSize?: number): number {
	const resolved = chunkSize ?? CHUNK_SIZE;
	if (!Number.isInteger(resolved) || resolved <= 0) {
		throw new Error("chunkSize must be a positive integer");
	}
	return resolved;
}

export function resolveKvPrefix(kvPrefix: number): number {
	if (!Number.isInteger(kvPrefix) || kvPrefix < 0 || kvPrefix > 255) {
		throw new Error("kvPrefix must be an integer between 0 and 255");
	}
	return kvPrefix;
}
