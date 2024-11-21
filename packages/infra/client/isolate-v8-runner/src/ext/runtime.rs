deno_core::extension!(
	rivet_runtime,
	deps = [
		rivet_kv
	],
	esm_entry_point = "ext:rivet_runtime/99_rivet_main.js",
	esm = [
		dir "js",
		"90_rivet_ns.js",
		"99_rivet_main.js"
	],
);
