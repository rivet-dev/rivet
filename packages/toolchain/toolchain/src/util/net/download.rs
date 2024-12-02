use anyhow::*;
use std::{
	fs::{self, File},
	io::{self, Write},
	path::Path,
};
use tempfile::TempDir;
use zip::ZipArchive;

/// Downloads a ZIP file, extracts a directory from it, and copies it to the destination.
pub fn zip(url: &str, src_dir_relative: &Path, dest_dir: &Path) -> Result<()> {
	ensure!(src_dir_relative.is_relative(), "src_dir must be relative");

	let temp_dir = TempDir::new()?;
	let temp_path = temp_dir.path().join("archive.zip");

	// Download the zip
	download(url, &temp_path)?;

	// Delete destination if it exists
	if dest_dir.is_dir() {
		fs::remove_dir_all(&dest_dir)?;
	}

	// Unzip the file
	extract(&temp_path, &src_dir_relative, &dest_dir)?;

	Ok(())
}

/// Downloads the ZIP file to a temp path.
fn download(url: &str, dest: &Path) -> Result<()> {
	let response = reqwest::blocking::get(url)
		.context("error fetching zip")?
		.error_for_status()
		.context("error status fetching zip")?;
	let bytes = response.bytes().context("failed to get zip body")?;
	let mut out = File::create(dest).with_context(|| {
		anyhow!(
			"failed to create download destionatin file: {}",
			dest.display()
		)
	})?;
	out.write_all(&bytes)?;

	Ok(())
}

/// Extracts the contents of the directory.
///
/// # Arguments
///
/// * `archive_path` - The path to the archive file.
/// * `inner_dir` - The directory inside the archive to copy from.
/// * `extract_to` - The path to extract the directory to.
fn extract(archive_path: &Path, inner_dir: &Path, extract_to: &Path) -> Result<()> {
	let file = fs::File::open(archive_path)
		.with_context(|| anyhow!("failed to open archive: {}", archive_path.display()))?;
	let mut archive = ZipArchive::new(file)?;

	for i in 0..archive.len() {
		let mut file = archive.by_index(i)?;
		let file_name = file.enclosed_name().context("unenclosed file name")?;

		// Filter files that are not in the src_dir
		let Result::Ok(file_name) = file_name.strip_prefix(inner_dir) else {
			continue;
		};

		// Build dest path
		let outpath = extract_to.join(file_name);

		// Copy file
		if file.name().ends_with('/') {
			fs::create_dir_all(&outpath)
				.with_context(|| anyhow!("failed create dir: {}", outpath.display()))?;
		} else {
			if let Some(p) = outpath.parent() {
				if !p.exists() {
					fs::create_dir_all(&p)
						.with_context(|| anyhow!("failed to create dir: {}", outpath.display()))?;
				}
			}
			let mut outfile = fs::File::create(&outpath)?;
			io::copy(&mut file, &mut outfile)?;
		}
	}

	Ok(())
}
