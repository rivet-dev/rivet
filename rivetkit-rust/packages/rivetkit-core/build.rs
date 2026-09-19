use std::env;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use fs_extra::dir;

// Stages the inspector-ui and inspector-tab bundles into `$OUT_DIR` so the
// inspector bundle module can embed both via include_dir!.
//
// Source resolution order per bundle:
//   1. In-crate `inspector-dist/<name>/`, staged by
//      `scripts/stage-inspector-bundle.mjs` before `cargo package` / publish.
//      This is the only copy that ships to crates.io, because `frontend/dist`
//      lives outside the crate and is therefore absent from the `.crate`
//      archive.
//   2. Monorepo `../../../frontend/dist/<name>/`, produced by the frontend
//      build. Used for in-workspace dev builds where staging has not run.
//   3. An empty placeholder, so a not-yet-built frontend degrades to a runtime
//      404 (`inspector.ui_asset_not_found`) instead of a compile error.
//
// A bundle only counts as present when its marker file exists, so an empty
// staging placeholder correctly falls through to the frontend build.
fn main() -> Result<()> {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = env::var("OUT_DIR")?;

	// Once any `cargo:rerun-if-changed` is emitted Cargo stops rerunning on
	// generic source changes, so the script itself must opt in explicitly.
	println!("cargo:rerun-if-changed=build.rs");

	stage_bundle(&manifest_dir, &out_dir, "inspector-ui", "index.html")?;
	stage_bundle(&manifest_dir, &out_dir, "inspector-tab", "styles.css")?;

	Ok(())
}

fn stage_bundle(manifest_dir: &str, out_dir: &str, name: &str, marker: &str) -> Result<()> {
	let manifest = Path::new(manifest_dir);
	let in_crate = manifest.join("inspector-dist").join(name);
	let monorepo = manifest.join("../../../frontend/dist").join(name);

	// Rerun when either candidate bundle changes so a rebuild picks up staged
	// or freshly built assets.
	println!("cargo:rerun-if-changed={}", in_crate.display());
	println!("cargo:rerun-if-changed={}", monorepo.display());

	let source = if in_crate.join(marker).is_file() {
		Some(in_crate)
	} else if monorepo.join(marker).is_file() {
		Some(monorepo)
	} else {
		None
	};

	let staged = Path::new(out_dir).join(name);
	if staged.exists() {
		fs::remove_dir_all(&staged)?;
	}
	fs::create_dir_all(&staged)?;

	match source {
		Some(source) => {
			let mut opts = dir::CopyOptions::new();
			opts.content_only = true;
			opts.overwrite = true;
			dir::copy(&source, &staged, &opts)
				.with_context(|| format!("failed to copy {} into OUT_DIR", source.display()))?;
		}
		None => {
			// Placeholder so include_dir! has something to embed even when
			// neither the staged crate bundle nor the frontend build exists yet.
			fs::write(staged.join(".empty"), b"")?;
		}
	}

	Ok(())
}
