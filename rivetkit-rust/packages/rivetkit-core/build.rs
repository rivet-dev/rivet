use std::env;
use std::fs;
use std::path::Path;

use anyhow::Result;
use fs_extra::dir;

// Stages frontend/dist/inspector-ui/ into $OUT_DIR/inspector-ui/ and
// frontend/dist/inspector-tab/ into $OUT_DIR/inspector-tab/ so the inspector
// bundle module can embed both via include_dir!. Falls back to an empty
// directory when the frontend has not been built yet, so a missing bundle
// degrades to 404 at runtime instead of a compile error.
fn main() -> Result<()> {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = env::var("OUT_DIR")?;

	// Once any `cargo:rerun-if-changed` is emitted Cargo stops rerunning on
	// generic source changes, so the script itself must opt in explicitly.
	println!("cargo:rerun-if-changed=build.rs");

	stage_dir(
		&manifest_dir,
		&out_dir,
		"../../../frontend/dist/inspector-ui",
		"inspector-ui",
	)?;
	stage_dir(
		&manifest_dir,
		&out_dir,
		"../../../frontend/dist/inspector-tab",
		"inspector-tab",
	)?;

	Ok(())
}

fn stage_dir(manifest_dir: &str, out_dir: &str, source_rel: &str, staged_name: &str) -> Result<()> {
	let source = Path::new(manifest_dir).join(source_rel);
	let staged = Path::new(out_dir).join(staged_name);

	println!("cargo:rerun-if-changed={}", source.display());

	if staged.exists() {
		fs::remove_dir_all(&staged)?;
	}
	fs::create_dir_all(&staged)?;

	if source.exists() && source.is_dir() {
		let mut opts = dir::CopyOptions::new();
		opts.content_only = true;
		opts.overwrite = true;
		dir::copy(&source, &staged, &opts)
			.unwrap_or_else(|e| panic!("failed to copy {source_rel} into OUT_DIR: {e}"));
	} else {
		// Placeholder so include_dir! has something to embed even when the
		// frontend has not been built yet.
		fs::write(staged.join(".empty"), b"")?;
	}

	Ok(())
}
