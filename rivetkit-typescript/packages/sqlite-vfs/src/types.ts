export interface KvVfsOptions {
	/** Get a single value by key. Returns null if missing. */
	get: (key: Uint8Array) => Promise<Uint8Array | null>;
	/** Get multiple values by keys. Returns null for missing keys. */
	getBatch: (keys: Uint8Array[]) => Promise<(Uint8Array | null)[]>;
	/** Put a single key-value pair */
	put: (key: Uint8Array, value: Uint8Array) => Promise<void>;
	/** Put multiple key-value pairs */
	putBatch: (entries: [Uint8Array, Uint8Array][]) => Promise<void>;
	/** Delete multiple keys */
	deleteBatch: (keys: Uint8Array[]) => Promise<void>;
}
