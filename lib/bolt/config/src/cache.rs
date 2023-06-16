use serde::{Deserialize, Serialize};
use std::{
	collections::{HashMap, HashSet},
	time::SystemTime,
};

/// Cached data used to speed up Bolt commands.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cache {
	/// Resolve image tags to their associated SHA tags.
	#[serde(default)]
	pub resolved_image_tags: HashMap<String, String>,

	/// If a file exists on S3, we will cache it. This assumes that files are
	/// never deleted from S3.
	#[serde(default)]
	pub s3_file_exists: HashSet<S3FileExistsEntry>,

	#[serde(default)]
	pub last_login_check: Option<SystemTime>,

	#[serde(default)]
	pub terraform_output_cache: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct S3FileExistsEntry {
	pub bucket: String,
	pub key: String,
}
