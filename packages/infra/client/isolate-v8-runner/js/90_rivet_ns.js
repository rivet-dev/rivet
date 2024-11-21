import * as kv from "ext:rivet_kv/40_rivet_kv.js";

const rivetNs = {
	kv: {
		get: kv.get,
		getBatch: kv.getBatch,
		list: kv.list,
		put: kv.put,
		putBatch: kv.putBatch,
		delete: kv.delete_,
		deleteBatch: kv.deleteBatch,
	},
};

export { rivetNs };
