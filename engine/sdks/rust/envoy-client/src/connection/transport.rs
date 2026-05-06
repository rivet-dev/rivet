//! Selects the public connection entrypoint from the effective transport cfg.
//!
//! Raw feature cfgs are not enough here because Cargo features are additive
//! and transport selection also depends on the build target.

#[cfg(all(
	target_arch = "wasm32",
	feature = "native-transport",
	feature = "wasm-transport"
))]
compile_error!(
	"`native-transport` and `wasm-transport` are mutually exclusive. Enable exactly one envoy-client transport."
);

#[cfg(not(any(feature = "native-transport", feature = "wasm-transport")))]
compile_error!(
	"rivet-envoy-client requires a WebSocket transport. Enable `native-transport` or `wasm-transport`."
);

#[cfg(rivet_envoy_native_transport)]
pub use super::native::start_connection;

#[cfg(rivet_envoy_wasm_transport)]
pub use super::wasm::start_connection;
