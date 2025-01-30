use anyhow::Result;
use vergen_git2::Emitter;

fn main() -> Result<()> {
	Emitter::default().emit()?;

	Ok(())
}
