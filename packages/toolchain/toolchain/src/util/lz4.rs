use anyhow::*;
use std::{
	fs::File,
	io::{BufReader, BufWriter},
	path::Path,
};

pub fn compress(input_path: &Path, output_path: &Path) -> Result<()> {
	let input_file = File::open(&input_path)?;
	let mut reader = BufReader::new(input_file);

	let output_file = File::create(&output_path)?;
	let writer = BufWriter::new(output_file);

	let mut encoder = lz4::EncoderBuilder::new().level(1).build(writer)?;

	// Pipe the bytes through the encoder
	std::io::copy(&mut reader, &mut encoder)?;

	encoder.finish().1?;

	Ok(())
}
