use fs_extra::dir;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
	let out_dir = env::var("OUT_DIR").unwrap();
	let ui_dir = Path::new(&out_dir).join("ui");
	let frontend_dist_path = Path::new(&manifest_dir).join("../../../frontend/dist");

	println!("cargo:rerun-if-changed={}", frontend_dist_path.display());

	if ui_dir.exists() {
		fs::remove_dir_all(&ui_dir).expect("Failed to remove existing ui directory");
	}

	if frontend_dist_path.exists() && frontend_dist_path.is_dir() {
		fs::create_dir_all(&ui_dir).expect("Failed to create ui directory");
		let mut copy_options = dir::CopyOptions::new();
		copy_options.content_only = true;
		dir::copy(&frontend_dist_path, &ui_dir, &copy_options)
			.expect("Failed to copy frontend/dist/ contents to ui directory");
	} else {
		// Create fallback ui directory with index.html
		fs::create_dir_all(&ui_dir).expect("Failed to create ui directory in OUT_DIR");

		let index_html_path = ui_dir.join("index.html");
		let index_html_content = r#"<!DOCTYPE html>
<html>
<head>
	<title>Rivet Engine</title>
</head>
<body>
	<h1>This build ships without the Rivet frontend.</h1>
	<p>If building from source, make sure to build <code>frontend/dist/</code> before building the Rivet Engine.</p>
</body>
</html>"#;

		fs::write(index_html_path, index_html_content).expect("Failed to write index.html");
	}
}
