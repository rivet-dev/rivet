/**
 * Input types matching the runner protocol PreloadedKv structure.
 * Defined locally to avoid a dependency on the runner-protocol package.
 */
export interface PreloadedKvInput {
	readonly entries: readonly {
		readonly key: ArrayBufferLike;
		readonly value: ArrayBufferLike;
	}[];
	readonly requestedGetKeys: readonly ArrayBufferLike[];
	readonly requestedPrefixes: readonly ArrayBufferLike[];
}

/**
 * Sorted array of [key, value] pairs for binary search lookups.
 * Used for prefix-based preloaded data (SQLite, connections, workflows).
 */
export type PreloadedEntries = [Uint8Array, Uint8Array][];

/**
 * Result of a preloaded key lookup. The value is null when the key was
 * requested but does not exist in storage.
 */
export interface PreloadHit {
	value: Uint8Array | null;
}

/**
 * Preloaded KV lookup supporting exact key lookup and prefix listing.
 *
 * get():
 * - PreloadHit: key was preloaded. `value` is the data, or null if absent.
 * - undefined: key was not preloaded, caller should fall back to KV read.
 *
 * listPrefix():
 * - [Uint8Array, Uint8Array][]: prefix was preloaded, these are the entries.
 * - undefined: prefix was not preloaded, caller should fall back to KV list.
 */
export interface PreloadMap {
	get(key: Uint8Array): PreloadHit | undefined;
	listPrefix(prefix: Uint8Array): [Uint8Array, Uint8Array][] | undefined;
}

/** Lexicographic comparison of two byte arrays. */
export function compareBytes(a: Uint8Array, b: Uint8Array): number {
	const len = Math.min(a.length, b.length);
	for (let i = 0; i < len; i++) {
		if (a[i] !== b[i]) return a[i] - b[i];
	}
	return a.length - b.length;
}

/** Binary search a sorted [key, value][] array. Returns the value if found, undefined otherwise. */
export function binarySearch(
	entries: PreloadedEntries,
	key: Uint8Array,
): Uint8Array | undefined {
	let lo = 0;
	let hi = entries.length - 1;
	while (lo <= hi) {
		const mid = (lo + hi) >>> 1;
		const cmp = compareBytes(entries[mid][0], key);
		if (cmp === 0) return entries[mid][1];
		if (cmp < 0) lo = mid + 1;
		else hi = mid - 1;
	}
	return undefined;
}

/**
 * Returns true if `key` starts with `prefix`.
 */
export function hasPrefix(key: Uint8Array, prefix: Uint8Array): boolean {
	if (key.length < prefix.length) return false;
	for (let i = 0; i < prefix.length; i++) {
		if (key[i] !== prefix[i]) return false;
	}
	return true;
}

/**
 * Binary search a sorted Uint8Array[] to check if a key exists.
 */
export function binarySearchExists(
	sortedKeys: Uint8Array[],
	key: Uint8Array,
): boolean {
	let lo = 0;
	let hi = sortedKeys.length - 1;
	while (lo <= hi) {
		const mid = (lo + hi) >>> 1;
		const cmp = compareBytes(sortedKeys[mid], key);
		if (cmp === 0) return true;
		if (cmp < 0) lo = mid + 1;
		else hi = mid - 1;
	}
	return false;
}

/**
 * Build a PreloadMap from pre-sorted Uint8Array data. This is the shared
 * core used by both `buildPreloadMap` (protocol input) and the engine
 * driver (already has Uint8Array arrays).
 *
 * All three arrays must already be sorted by `compareBytes`.
 */
export function createPreloadMap(
	sortedEntries: PreloadedEntries,
	sortedGetKeys: Uint8Array[],
	sortedPrefixes: Uint8Array[],
): PreloadMap {
	return {
		get(key: Uint8Array): PreloadHit | undefined {
			const value = binarySearch(sortedEntries, key);
			if (value !== undefined) return { value };

			if (binarySearchExists(sortedGetKeys, key)) return { value: null };

			return undefined;
		},

		listPrefix(prefix: Uint8Array): [Uint8Array, Uint8Array][] | undefined {
			if (!binarySearchExists(sortedPrefixes, prefix)) {
				return undefined;
			}

			const result: [Uint8Array, Uint8Array][] = [];
			let lo = 0;
			let hi = sortedEntries.length - 1;

			// Binary search to find the first entry >= prefix.
			while (lo <= hi) {
				const mid = (lo + hi) >>> 1;
				if (compareBytes(sortedEntries[mid][0], prefix) < 0) {
					lo = mid + 1;
				} else {
					hi = mid - 1;
				}
			}

			// Scan forward from `lo` collecting all entries with the prefix.
			for (let i = lo; i < sortedEntries.length; i++) {
				if (!hasPrefix(sortedEntries[i][0], prefix)) break;
				result.push(sortedEntries[i]);
			}

			return result;
		},
	};
}

/**
 * Build a PreloadMap from protocol data (ArrayBuffer-based).
 *
 * Returns undefined if the protocol data is undefined/null (no preloading).
 */
export function buildPreloadMap(
	preloadedKv: PreloadedKvInput | null | undefined,
): PreloadMap | undefined {
	if (preloadedKv == null) return undefined;

	const sorted: PreloadedEntries = preloadedKv.entries
		.map(
			(entry) =>
				[
					new Uint8Array(entry.key),
					new Uint8Array(entry.value),
				] as [Uint8Array, Uint8Array],
		)
		.sort((a, b) => compareBytes(a[0], b[0]));

	const requestedGetKeys: Uint8Array[] = preloadedKv.requestedGetKeys
		.map((k) => new Uint8Array(k))
		.sort(compareBytes);

	const requestedPrefixes: Uint8Array[] = preloadedKv.requestedPrefixes
		.map((k) => new Uint8Array(k))
		.sort(compareBytes);

	return createPreloadMap(sorted, requestedGetKeys, requestedPrefixes);
}
