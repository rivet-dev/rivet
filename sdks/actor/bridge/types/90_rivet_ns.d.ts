// DO NOT MODIFY
//
// Generated from sdks/actor-bridge/

import type { Metadata } from "./types/metadata.d.ts";
export type { Metadata } from "./types/metadata.d.ts";
export declare function deepFreeze<T extends object>(object: T): Readonly<T>;
export declare const ACTOR_CONTEXT: {
	metadata: Metadata;
	kv: {
		get: <K, V>(
			key: K,
			options?: import("./40_rivet_kv.d.ts").GetOptions,
		) => Promise<V | null>;
		getBatch: <K extends Array<unknown>, V>(
			keys: K,
			options?: import("./40_rivet_kv.d.ts").GetBatchOptions,
		) => Promise<Map<K[number], V>>;
		list: <K, V>(
			options?: import("./40_rivet_kv.d.ts").ListOptions<K>,
		) => Promise<Map<K, V>>;
		put: <K, V>(
			key: K,
			value: V | ArrayBuffer,
			options?: import("./40_rivet_kv.d.ts").PutOptions,
		) => Promise<void>;
		putBatch: <K, V>(
			obj: Map<K, V | ArrayBuffer>,
			options?: import("./40_rivet_kv.d.ts").PutBatchOptions,
		) => Promise<void>;
		delete: <K>(key: K) => Promise<void>;
		deleteBatch: <K extends Array<unknown>>(keys: K) => Promise<void>;
		deleteAll: () => Promise<void>;
	};
};
