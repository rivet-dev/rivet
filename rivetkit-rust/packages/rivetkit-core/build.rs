use std::env;

fn main() {
	println!("cargo:rustc-check-cfg=cfg(rivetkit_native_runtime)");
	println!("cargo:rustc-check-cfg=cfg(rivetkit_wasm_runtime)");

	let native_runtime = env::var_os("CARGO_FEATURE_NATIVE_RUNTIME").is_some();
	let wasm_runtime = env::var_os("CARGO_FEATURE_WASM_RUNTIME").is_some();

	// Use custom cfgs instead of raw feature flags because Cargo features are
	// additive, so native and wasm features can be enabled at the same time.
	// These cfgs collapse that feature set into exactly one effective runtime.
	if wasm_runtime && !native_runtime {
		println!("cargo:rustc-cfg=rivetkit_wasm_runtime");
	} else {
		println!("cargo:rustc-cfg=rivetkit_native_runtime");
	}
}
