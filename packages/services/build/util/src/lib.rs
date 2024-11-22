use std::hash::{DefaultHasher, Hasher};

use proto::backend;
use rivet_operation::prelude::*;

/// Generates the file name that holds the build tar.
pub fn file_name(
	kind: backend::build::BuildKind,
	compression: backend::build::BuildCompression,
) -> String {
	let file_name = match kind {
		backend::build::BuildKind::DockerImage => "image",
		backend::build::BuildKind::OciBundle => "oci-bundle",
		backend::build::BuildKind::JavaScript => "js-bundle",
	};
	let file_ext = "tar";
	let file_ext_compression = match compression {
		backend::build::BuildCompression::None => "",
		backend::build::BuildCompression::Lz4 => ".lz4",
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
