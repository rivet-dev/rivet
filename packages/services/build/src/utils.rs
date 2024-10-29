use std::hash::{DefaultHasher, Hasher};

use chirp_workflow::prelude::*;

use crate::types::{BuildCompression, BuildKind};

/// Generates the file name that holds the build tar.
pub fn file_name(kind: BuildKind, compression: BuildCompression) -> String {
	let file_name = match kind {
		BuildKind::DockerImage => "image",
		BuildKind::OciBundle => "oci-bundle",
		BuildKind::JavaScript => "index",
	};
	let file_ext = match kind {
		BuildKind::DockerImage | BuildKind::OciBundle => "tar",
		BuildKind::JavaScript => "js",
	};
	let file_ext_compression = match compression {
		BuildCompression::None => "",
		BuildCompression::Lz4 => ".lz4",
	};
	format!("{file_name}.{file_ext}{file_ext_compression}")
}

pub fn build_hash(build_id: Uuid) -> u64 {
	// Hash build so that the ATS server that we download the build from is always the same one. This
	// improves cache hit rates and reduces download times.
	let mut hasher = DefaultHasher::new();
	hasher.write(build_id.as_bytes());
	hasher.finish()
}
