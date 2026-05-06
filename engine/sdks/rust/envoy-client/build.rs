use std::env;

fn main() {
	println!("cargo:rustc-check-cfg=cfg(rivet_envoy_native_transport)");
	println!("cargo:rustc-check-cfg=cfg(rivet_envoy_wasm_transport)");

	let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
	let native_transport = env::var_os("CARGO_FEATURE_NATIVE_TRANSPORT").is_some();
	let wasm_transport = env::var_os("CARGO_FEATURE_WASM_TRANSPORT").is_some();
	let is_wasm = target_arch == "wasm32";

	// Use custom cfgs instead of raw feature flags because Cargo features are
	// additive, so native and wasm transport features can be enabled together.
	// These cfgs collapse features plus target_arch into one effective transport.
	if native_transport && !is_wasm {
		println!("cargo:rustc-cfg=rivet_envoy_native_transport");
	} else if wasm_transport {
		println!("cargo:rustc-cfg=rivet_envoy_wasm_transport");
	}
}
