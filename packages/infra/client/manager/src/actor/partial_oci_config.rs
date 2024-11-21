use serde::Deserialize;

/// Partial configuration file structure for deserializing the user-provided OCI config.json.
#[derive(Deserialize)]
pub struct PartialOciConfig {
	pub process: PartialOciConfigProcess,
}

#[derive(Deserialize)]
pub struct PartialOciConfigProcess {
	pub args: Vec<String>,
	pub env: Vec<String>,
	pub user: String,
	pub cwd: String,
}
