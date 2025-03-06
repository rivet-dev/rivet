use serde::{Deserialize, Serialize};

/// Partial configuration file structure for deserializing the user-provided OCI config.json.
#[derive(Deserialize)]
pub struct PartialOciConfig {
	pub process: PartialOciConfigProcess,
}

#[derive(Deserialize)]
pub struct PartialOciConfigProcess {
	pub args: Vec<String>,
	pub env: Vec<String>,
	pub user: PartialOciConfigUser,
	pub cwd: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialOciConfigUser {
	uid: i32,
	gid: i32,
	umask: Option<i32>,
	additional_gids: Option<Vec<i32>>,
}
