use anyhow::Result;

fn main() -> Result<()> {
	// Configure vergen to emit build metadata
	vergen::Emitter::default()
		.add_instructions(&vergen::BuildBuilder::all_build()?)?
		.add_instructions(&vergen::CargoBuilder::all_cargo()?)?
		.add_instructions(&vergen::RustcBuilder::all_rustc()?)?
		.add_instructions(&vergen_gitcl::GitclBuilder::all_git()?)?
		.emit()?;

	println!("cargo:rerun-if-env-changed=OVERRIDE_GIT_SHA");

	if let Ok(git_sha) = std::env::var("OVERRIDE_GIT_SHA") {
		println!("cargo:rustc-env=VERGEN_GIT_SHA={git_sha}");
	}

	Ok(())
}
