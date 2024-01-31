use proto::backend::{self};
use rivet_operation::prelude::*;

/// Generates the file name that holds the build tar.
pub fn file_name(
	kind: backend::build::BuildKind,
	compression: backend::build::BuildCompression,
) -> String {
	let file_name = match kind {
		backend::build::BuildKind::DockerImage => "image",
		backend::build::BuildKind::OciBundle => "oci-bundle",
	};
	let file_ext = match compression {
		backend::build::BuildCompression::None => "tar",
		backend::build::BuildCompression::Lz4 => "tar.lz4",
	};
	format!("{file_name}.{file_ext}")
}
