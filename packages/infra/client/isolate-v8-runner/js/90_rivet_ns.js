import * as kv from "ext:rivet_kv/40_rivet_kv.js";

const rivetNs = {
	kv: {
		get: kv.get,
		getBatch: kv.getBatch,
	},
};

export { rivetNs };
