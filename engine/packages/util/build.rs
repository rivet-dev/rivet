use anyhow::Result;

fn main() -> Result<()> {
	// Configure vergen to emit build metadata
	vergen::Emitter::default()
		.add_instructions(&vergen::BuildBuilder::all_build()?)?
		.add_instructions(&vergen::CargoBuilder::all_cargo()?)?
		.add_instructions(&vergen::RustcBuilder::all_rustc()?)?
		.add_instructions(&vergen_gitcl::GitclBuilder::all_git()?)?
		.emit()?;

	Ok(())
}
