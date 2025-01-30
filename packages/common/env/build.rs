use anyhow::Result;
use vergen::Emitter;

fn main() -> Result<()> {
	Emitter::default().emit()?;

	Ok(())
}
