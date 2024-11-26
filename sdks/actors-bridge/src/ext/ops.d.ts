/**
 * This file contains types for Rivet operations.
 */

declare module "ext:core/ops" {
	import type { InKey, OutKey, OutEntry, ListQuery } from "internal_types";

	export function op_rivet_kv_get(key: InKey): Promise<OutEntry | null>;
	export function op_rivet_kv_get_batch(keys: InKey[]): Promise<Map<OutKey, OutEntry>>;
	export function op_rivet_kv_list(
		query: ListQuery,
		reverse: boolean,
		limit?: number
	): Promise<Map<OutKey, OutEntry>>;
	export function op_rivet_kv_put(key: InKey, value: Uint8Array): Promise<void>;
	export function op_rivet_kv_put_batch(entries: Map<InKey, Uint8Array>): Promise<void>;
	export function op_rivet_kv_delete(key: InKey): Promise<void>;
	export function op_rivet_kv_delete_batch(keys: InKey[]): Promise<void>;
	export function op_rivet_kv_delete_all(): Promise<void>;
}
