pub mod peer;
pub mod public;

pub const TEST_ADMIN_TOKEN: &str = "default";

/// Helper function to format endpoint URL from port
pub fn get_endpoint(port: u16) -> String {
	format!("http://127.0.0.1:{}", port)
}
