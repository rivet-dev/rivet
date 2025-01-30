use anyhow::Result;
use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder};

fn main() -> Result<()> {
	Emitter::default()
		.add_instructions(&BuildBuilder::all_build()?)?
		.add_instructions(&CargoBuilder::all_cargo()?)?
		.add_instructions(&RustcBuilder::all_rustc()?)?
		.emit()?;

	Ok(())
}
