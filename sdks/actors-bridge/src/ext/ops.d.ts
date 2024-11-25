/**
 * This file contains types for Rivet operations.
 */

declare module "ext:core/ops" {
  export type Key = any;
  export type Entry = any;
  export type ListQuery = any;

  export function op_rivet_kv_get(key: Key): Promise<Entry | null>;
  export function op_rivet_kv_get_batch(keys: Key[]): Promise<Map<Key, Entry>>;
  export function op_rivet_kv_list(
    query: ListQuery,
    reverse: boolean,
    limit?: number,
  ): Promise<Map<Key, Entry>>;
  export function op_rivet_kv_put(key: Key, value: any): Promise<void>;
  export function op_rivet_kv_put_batch(
    entries: Map<Key, JsBuffer>,
  ): Promise<void>;
  export function op_rivet_kv_delete(key: Key): Promise<void>;
  export function op_rivet_kv_delete_batch(keys: Key[]): Promise<void>;
  export function op_rivet_kv_delete_all(): Promise<void>;
}
