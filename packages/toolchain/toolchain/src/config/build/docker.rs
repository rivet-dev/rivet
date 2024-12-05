use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use super::Compression;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Build {
	/// Existing image tag to upload.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub image: Option<String>,
	/// Dockerfile to build.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dockerfile: Option<String>,
	/// Directory to build the Docker image from.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub build_path: Option<String>,
	/// Build target to upload.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub build_target: Option<String>,
	/// Build arguments to pass to the build.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub build_args: Option<HashMap<String, String>>,
	/// Unstable features.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unstable: Option<Unstable>,
}

impl Build {
	pub fn unstable(&self) -> Unstable {
		self.unstable.clone().unwrap_or_default()
	}
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Unstable {
	pub allow_root: Option<bool>,
	pub build_method: Option<BuildMethod>,
	pub bundle: Option<BundleKind>,
	pub compression: Option<Compression>,
}

impl Unstable {
	pub fn allow_root(&self) -> bool {
		self.allow_root.unwrap_or(false)
	}

	pub fn build_method(&self) -> BuildMethod {
		self.build_method.unwrap_or(BuildMethod::Buildx)
	}

	pub fn bundle(&self) -> BundleKind {
		self.bundle.unwrap_or(BundleKind::OciBundle)
	}
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BuildMethod {
	/// Use the native Docker build command. Only used if Buildx is not available.
	Buildx,

	/// Create & use a Buildx builder on this machine. Required for cross-platform compilation.
	Native,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, strum::AsRefStr, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BundleKind {
	/// Legacy option. Docker image archive output from `docker save`. Slower lobby start
	/// times.
	#[strum(serialize = "docker_image")]
	DockerImage,

	/// OCI bundle archive derived from a generated Docker image. Optimized for fast lobby start
	/// times.
	#[strum(serialize = "oci_bundle")]
	OciBundle,
}
