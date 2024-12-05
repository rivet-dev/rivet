deno_core::extension!(
	rivet_runtime,
	deps = [
		rivet_kv
	],
	esm_entry_point = "ext:rivet_runtime/90_rivet_ns.js",
	esm = [
		dir "js",
		"90_rivet_ns.js"
	],
);
