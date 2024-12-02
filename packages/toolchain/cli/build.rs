use anyhow::Result;
use vergen_git2::{BuildBuilder, CargoBuilder, Emitter, Git2Builder, RustcBuilder};

fn main() -> Result<()> {
	Emitter::default()
		.add_instructions(&BuildBuilder::all_build()?)?
		.add_instructions(&CargoBuilder::all_cargo()?)?
		.add_instructions(&Git2Builder::default().sha(true).branch(true).build()?)?
		.add_instructions(&RustcBuilder::all_rustc()?)?
		.emit()?;

	Ok(())
}
