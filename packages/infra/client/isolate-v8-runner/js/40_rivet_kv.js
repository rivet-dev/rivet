import { op_rivet_kv_get, op_rivet_kv_get_batch } from "ext:core/ops";

async function get(key, options) {
	return (await op_rivet_kv_get(key, options)) ?? undefined;
}

async function getBatch(keys, options) {
	return (await op_rivet_kv_get_batch(keys, options)) ?? undefined;
}

export { get, getBatch };
