use anyhow::*;
use rivet_api::models::{BuildsBuildCompression, BuildsBuildKind};
use std::path::Path;

use crate::{config, util::lz4};

/// Generates the file name that holds the build tar.
pub fn file_name(kind: BuildsBuildKind, compression: BuildsBuildCompression) -> String {
	let file_name = match kind {
		BuildsBuildKind::DockerImage => "image",
		BuildsBuildKind::OciBundle => "oci-bundle",
		BuildsBuildKind::Javascript => "js-bundle",
	};
	let file_ext = "tar";
	let file_ext_compression = match compression {
		BuildsBuildCompression::None => "",
		BuildsBuildCompression::Lz4 => ".lz4",
	};
	format!("{file_name}.{file_ext}{file_ext_compression}")
}

/// Compresses a given file with the given build compression. This is used as the last step in the
/// build process before uploading the output file.
pub async fn compress_build(
	input_path: &Path,
	compression: config::build::Compression,
) -> Result<tempfile::TempPath> {
	// Compress the bundle
	let compressed_file = tempfile::NamedTempFile::new()?;
	let compressed_file_path = compressed_file.into_temp_path();
	match compression {
		config::build::Compression::None => {
			tokio::fs::rename(&input_path, &compressed_file_path).await?;
		}
		config::build::Compression::Lz4 => {
			let input_path = input_path.to_owned();
			let compressed_file_path = compressed_file_path.to_owned();
			tokio::task::spawn_blocking(move || lz4::compress(&input_path, &compressed_file_path))
				.await??;
		}
	}

	Ok(compressed_file_path)
}
