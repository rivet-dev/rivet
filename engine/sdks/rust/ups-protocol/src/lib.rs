pub mod generated;
pub mod versioned;

// Re-export latest
pub use generated::v3::*;

// TODO: Temp dynamic version
pub static PROTOCOL_VERSION: std::sync::LazyLock<u16> = std::sync::LazyLock::new(|| {
	std::env::var("RIVET_UPS_VERSION")
		.ok()
		.and_then(|x| x.parse::<u16>().ok())
		.unwrap_or(generated::PROTOCOL_VERSION)
});
// pub use generated::PROTOCOL_VERSION;
