use anyhow::*;

pub fn indent_string(s: &str, indent: impl AsRef<str>, trim_start: bool) -> String {
	let mut out = String::with_capacity(s.len());
	let mut iter = s.split("\n");

	if let Some(chunk) = iter.next() {
		if !trim_start {
			out.push_str(indent.as_ref());
		}

		out.push_str(chunk);
	}

	for chunk in iter {
		out.push('\n');
		out.push_str(indent.as_ref());
		out.push_str(chunk);
	}

	out
}

pub fn chunk_string(s: &str, size: usize) -> Vec<String> {
	s.as_bytes()
		.chunks(size)
		.map(|chunk| String::from_utf8_lossy(chunk).to_string())
		.collect()
}

pub fn colored_json(value: &serde_json::Value) -> Result<String> {
	colored_json_inner(value, colored_json::PrettyFormatter::new())
}

pub fn colored_json_ugly(value: &serde_json::Value) -> Result<String> {
	colored_json_inner(value, colored_json::CompactFormatter {})
}

fn colored_json_inner<T: serde_json::ser::Formatter>(
	value: &serde_json::Value,
	formatter: T,
) -> Result<String> {
	use colored_json::{ColorMode, ColoredFormatter, Output, Style, Styler};
	use serde::Serialize;

	let mut writer = Vec::<u8>::with_capacity(128);

	let mode = ColorMode::Auto(Output::StdOut);
	if mode.use_color() {
		let formatter = ColoredFormatter::with_styler(
			formatter,
			Styler {
				object_brackets: Style::new(),
				array_brackets: Style::new(),
				integer_value: Style::new().yellow(),
				bool_value: Style::new().magenta(),
				..Default::default()
			},
		);

		let mut serializer = serde_json::Serializer::with_formatter(&mut writer, formatter);
		value.serialize(&mut serializer)?;
	} else {
		let mut serializer = serde_json::Serializer::with_formatter(&mut writer, formatter);
		value.serialize(&mut serializer)?;
	}

	Ok(String::from_utf8_lossy(&writer).to_string())
}
