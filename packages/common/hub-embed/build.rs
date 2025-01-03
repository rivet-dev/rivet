use std::{env, fs, path::Path};

const HUB_URL: &str = "https://releases.rivet.gg/hub/2025-01-03-02-30-43-9f9c4fd-embed.zip";
const OUT_DIR: &str = "hub_files";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Get the output directory from the cargo environment variable
	let target_dir = env::var("OUT_DIR")?;
	let out_dir = Path::new(&target_dir).join(OUT_DIR);

	// Download the file
	println!("cargo:rerun-if-changed=build.rs");

	let response = reqwest::blocking::get(HUB_URL)?;
	let zip_content = response.bytes()?;

	// Extract to out dir
	if !out_dir.exists() {
		fs::create_dir_all(&out_dir)?;
	}

	let mut zip_archive = zip::ZipArchive::new(std::io::Cursor::new(zip_content))?;

	for i in 0..zip_archive.len() {
		let mut file = zip_archive.by_index(i)?;
		let outpath = out_dir.join(file.name());

		if file.name().ends_with('/') {
			fs::create_dir_all(&outpath)?;
		} else {
			if let Some(p) = outpath.parent() {
				if !p.exists() {
					fs::create_dir_all(p)?;
				}
			}
			let mut outfile = fs::File::create(&outpath)?;
			std::io::copy(&mut file, &mut outfile)?;
		}
	}

	// Set the path in the env
	println!("cargo:rustc-env=HUB_PATH={}", out_dir.display());

	Ok(())
}
