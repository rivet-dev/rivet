use std::{fmt, result::Result::Ok, str::FromStr};

use anyhow::*;
use clap::ValueEnum;
use foundationdb::tuple::{PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset};
use rivet_term::console::style;
use uuid::Uuid;

use crate::util::format::colored_json;

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq)]
pub enum ListStyle {
	List,
	Tree,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimpleTupleValue {
	U64(u64),
	I64(i64),
	F64(f64),
	Uuid(Uuid),
	String(String),
	Bytes(Vec<u8>),
}

impl SimpleTupleValue {
	fn parse(value: &str) -> Self {
		if let Ok(v) = value.parse::<i64>() {
			SimpleTupleValue::I64(v)
		} else if let Ok(v) = value.parse::<u64>() {
			SimpleTupleValue::U64(v)
		} else if let Ok(v) = value.parse::<f64>() {
			SimpleTupleValue::F64(v)
		} else if let Ok(v) = Uuid::from_str(value) {
			SimpleTupleValue::Uuid(v)
		} else {
			SimpleTupleValue::String(unescape(value))
		}
	}
}

impl fmt::Display for SimpleTupleValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			SimpleTupleValue::U64(v) => write!(f, "{}", style(v).cyan()),
			SimpleTupleValue::I64(v) => write!(f, "{}", style(v).magenta()),
			SimpleTupleValue::F64(v) => write!(f, "{}", style(v).red()),
			SimpleTupleValue::Uuid(v) => write!(f, "{}", style(v).blue()),
			SimpleTupleValue::String(v) => {
				if v.is_empty() {
					write!(f, "{}", style("<empty>").dim())
				} else {
					write!(f, "{}", style(v).green())
				}
			}
			SimpleTupleValue::Bytes(v) => {
				let hex_string = if v.len() > 512 { &v[..512] } else { v }
					.iter()
					.map(|byte| format!("{:02x}", byte))
					.collect::<String>();
				write!(f, "{}", style(hex_string).italic())?;

				if v.len() > 512 {
					write!(
						f,
						"{} {}",
						style("...").italic(),
						style(format!("({} bytes)", v.len())).dim()
					)?;
				}

				Ok(())
			}
		}
	}
}

impl TuplePack for SimpleTupleValue {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		match self {
			SimpleTupleValue::U64(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::I64(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::F64(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::Uuid(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::String(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::Bytes(v) => {
				w.write_all(v)?;
				Ok(VersionstampOffset::None {
					size: u32::try_from(v.len())
						.map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
				})
			}
		}
	}
}

impl<'de> TupleUnpack<'de> for SimpleTupleValue {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		if let Ok((input, v)) = <i64>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::I64(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <f64>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::F64(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <Uuid>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::Uuid(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <String>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::String(v);
			Ok((input, v))
		} else {
			let v = SimpleTupleValue::Bytes(input.to_vec());
			Ok((&input[0..0], v))
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimpleValue {
	U64(u64),
	F64(f64),
	I64(i64),
	Uuid(Uuid),
	Json(serde_json::Value),
	String(String),
	Bytes(Vec<u8>),
}

impl SimpleValue {
	pub fn parse_bytes(type_hint: Option<&str>, value: &[u8]) -> Result<Self> {
		let parsed_value = match type_hint {
			Some("u64") => SimpleValue::U64(u64::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as u64"))?,
			)),
			Some("i64") => SimpleValue::I64(i64::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as i64"))?,
			)),
			Some("f64") => SimpleValue::F64(f64::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as f64"))?,
			)),
			Some("uuid") => Uuid::from_slice(value)
				.map(SimpleValue::Uuid)
				.with_context(|| format!("Could not parse `{value:?}` as UUID"))?,
			Some("json") => {
				let s = std::str::from_utf8(value)
					.with_context(|| format!("Could not parse `{value:?}` as JSON"))?;
				let v = serde_json::from_str::<serde_json::Value>(s)
					.with_context(|| format!("Could not parse `{value:?}` as JSON"))?;
				SimpleValue::Json(v)
			}
			Some("str") => std::str::from_utf8(value)
				.map(|x| x.to_string())
				.map(SimpleValue::String)
				.with_context(|| format!("Could not parse `{value:?}` as string"))?,
			Some("bytes") | Some("b") => SimpleValue::Bytes(value.to_vec()),
			Some(type_hint) => bail!("unknown type: `{type_hint}`"),
			_ => {
				if let Ok(value) = value.try_into() {
					SimpleValue::I64(i64::from_be_bytes(value))
				} else if let Ok(v) = Uuid::from_slice(value) {
					SimpleValue::Uuid(v)
				} else if let Ok(v) = serde_json::from_slice(value) {
					SimpleValue::Json(v)
				} else if let Ok(v) = std::str::from_utf8(value) {
					SimpleValue::String(v.to_string())
				} else {
					SimpleValue::Bytes(value.to_vec())
				}
			}
		};

		Ok(parsed_value)
	}

	pub fn parse_str(type_hint: Option<&str>, value: &str) -> Result<Self> {
		let mut escaped = false;

		let mut chars = value.chars().enumerate();

		let (type_hint, value) = if type_hint.is_some() {
			(type_hint, value)
		} else {
			let prefix_end_idx = loop {
				let Some((i, c)) = chars.next() else {
					break None;
				};

				match c {
					'\\' => escaped = !escaped,
					':' if !escaped => break Some(i),
					_ => escaped = false,
				}
			};

			let type_hint = prefix_end_idx.map(|x| &value[..x]);
			let value = &value[prefix_end_idx.map(|x| x + 1).unwrap_or_default()..];

			(type_hint, value)
		};

		let parsed_value = match type_hint {
			Some("u64") => value
				.parse::<u64>()
				.map(SimpleValue::U64)
				.with_context(|| format!("Could not parse `{value}` as u64"))?,
			Some("i64") => value
				.parse::<i64>()
				.map(SimpleValue::I64)
				.with_context(|| format!("Could not parse `{value}` as i64"))?,
			Some("f64") => value
				.parse::<f64>()
				.map(SimpleValue::F64)
				.with_context(|| format!("Could not parse `{value}` as f64"))?,
			Some("uuid") => Uuid::from_str(value)
				.map(SimpleValue::Uuid)
				.with_context(|| format!("Could not parse `{value}` as UUID"))?,
			Some("json") => {
				let v = serde_json::from_str::<serde_json::Value>(value)
					.with_context(|| format!("Could not parse `{value}` as JSON"))?;
				let s = serde_json::to_string(&v)
					.with_context(|| format!("Could not parse `{value}` as JSON"))?;
				SimpleValue::String(s)
			}
			Some("str") => SimpleValue::String(value.to_string()),
			Some("bytes") | Some("b") => {
				let bytes = hex::decode(value.as_bytes())
					.with_context(|| format!("Could not parse `{value}` as hex encoded bytes"))?;
				SimpleValue::Bytes(bytes)
			}
			Some(type_hint) => bail!("unknown type: `{type_hint}`"),
			_ => SimpleTupleValue::parse(value).into(),
		};

		Ok(parsed_value)
	}

	pub fn serialize(&self) -> Result<Vec<u8>> {
		let v = match self {
			SimpleValue::U64(v) => v.to_be_bytes().to_vec(),
			SimpleValue::I64(v) => v.to_be_bytes().to_vec(),
			SimpleValue::F64(v) => v.to_be_bytes().to_vec(),
			SimpleValue::Uuid(v) => v.as_bytes().to_vec(),
			SimpleValue::Json(v) => serde_json::to_vec(v)?,
			SimpleValue::String(v) => v.as_bytes().to_vec(),
			SimpleValue::Bytes(v) => v.clone(),
		};

		Ok(v)
	}
}

impl fmt::Display for SimpleValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			SimpleValue::U64(v) => write!(f, "{}", style(v).cyan()),
			SimpleValue::I64(v) => write!(f, "{}", style(v).magenta()),
			SimpleValue::F64(v) => write!(f, "{}", style(v).red()),
			SimpleValue::Uuid(v) => write!(f, "{}", style(v).blue()),
			SimpleValue::Json(v) => {
				if let Ok(json) = colored_json(v) {
					write!(f, "{json}")
				} else {
					write!(f, "{}", style(v).yellow())
				}
			}
			SimpleValue::String(v) => write!(f, "{}", style(v).green()),
			SimpleValue::Bytes(v) => {
				let hex_string = if v.len() > 512 { &v[..512] } else { v }
					.iter()
					.map(|byte| format!("{:02x}", byte))
					.collect::<String>();
				write!(f, "{}", style(hex_string).italic())?;

				if v.len() > 512 {
					write!(
						f,
						"{} {}",
						style("...").italic(),
						style(format!("({} bytes)", v.len())).dim()
					)?;
				}

				Ok(())
			}
		}
	}
}

impl From<SimpleTupleValue> for SimpleValue {
	fn from(value: SimpleTupleValue) -> Self {
		match value {
			SimpleTupleValue::U64(v) => SimpleValue::U64(v),
			SimpleTupleValue::I64(v) => SimpleValue::I64(v),
			SimpleTupleValue::F64(v) => SimpleValue::F64(v),
			SimpleTupleValue::Uuid(v) => SimpleValue::Uuid(v),
			SimpleTupleValue::String(v) => SimpleValue::String(v),
			SimpleTupleValue::Bytes(v) => SimpleValue::Bytes(v),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleTupleSegment {
	value: SimpleTupleValue,
}

impl SimpleTupleSegment {
	fn parse(prefix: Option<&str>, value: &str) -> Result<Self> {
		let parsed_value = match prefix {
			Some("u64") => value
				.parse::<u64>()
				.map(SimpleTupleValue::U64)
				.with_context(|| format!("Could not parse `{value}` as u64"))?,
			Some("i64") => value
				.parse::<i64>()
				.map(SimpleTupleValue::I64)
				.with_context(|| format!("Could not parse `{value}` as i64"))?,
			Some("f64") => value
				.parse::<f64>()
				.map(SimpleTupleValue::F64)
				.with_context(|| format!("Could not parse `{value}` as f64"))?,
			Some("uuid") => Uuid::from_str(value)
				.map(SimpleTupleValue::Uuid)
				.with_context(|| format!("Could not parse `{value}` as UUID"))?,
			Some("bytes") | Some("b") => {
				let bytes = hex::decode(value.as_bytes())
					.with_context(|| format!("Could not parse `{value}` as hex encoded bytes"))?;
				SimpleTupleValue::Bytes(bytes)
			}
			Some("str") => SimpleTupleValue::String(value.to_string()),
			Some(prefix) => bail!("unknown type: `{prefix}`"),
			_ => SimpleTupleValue::parse(value),
		};

		Ok(SimpleTupleSegment {
			value: parsed_value,
		})
	}
}

impl fmt::Display for SimpleTupleSegment {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.value)
	}
}

impl TuplePack for SimpleTupleSegment {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		self.value.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SimpleTupleSegment {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, v) = SimpleTupleValue::unpack(input, tuple_depth)?;
		let v = SimpleTupleSegment { value: v };

		Ok((input, v))
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleTuple {
	pub segments: Vec<SimpleTupleSegment>,
}

impl SimpleTuple {
	pub fn new() -> Self {
		SimpleTuple {
			segments: Vec::new(),
		}
	}

	pub fn slice(&self, n: usize) -> Self {
		SimpleTuple {
			segments: self.segments.iter().take(n).cloned().collect(),
		}
	}

	pub fn parse(value: &str) -> Result<(Self, bool, usize)> {
		let mut segments = Vec::new();
		let mut back_count = 0;
		let mut normal_segment_encountered = false;
		let mut start = 0;
		let mut prefix = None;
		let mut escaped = false;

		let chars = value
			.chars()
			.chain((!value.ends_with('/')).then_some('/'))
			.enumerate();
		for (i, c) in chars {
			match c {
				'/' => {
					if i > start {
						let segment = &value[start..i];

						// Parse back
						if segment == ".." {
							if normal_segment_encountered {
								bail!("Invalid path: '..' cannot go after other segments");
							}

							back_count += 1;
						} else if segment == "." {
							// Noop
						} else {
							// Parse normal segment
							normal_segment_encountered = true;
							segments
								.push(SimpleTupleSegment::parse(prefix.take(), segment.trim())?);
						}
					} else if start != 0 && i == start {
						segments.push(SimpleTupleSegment::parse(prefix.take(), "")?);
					}

					start = i + 1;
				}
				'\\' => escaped = !escaped,
				':' if !escaped && prefix.is_none() => {
					prefix = Some(&value[start..i]);
					start = i + 1;
				}
				_ => escaped = false,
			}
		}

		Ok((
			SimpleTuple { segments },
			!value.starts_with('/'),
			back_count,
		))
	}

	pub fn print(&self, list_style: &ListStyle, last_key: &SimpleTuple) {
		match list_style {
			ListStyle::List => {
				print!("  {self}");
			}
			ListStyle::Tree => {
				let common_prefix_len = self
					.segments
					.iter()
					.zip(&last_key.segments)
					.take_while(|(a, b)| a == b)
					.count();

				for (i, segment) in self.segments.iter().skip(common_prefix_len).enumerate() {
					print!(
						"  {}/{}",
						style("| ".repeat(common_prefix_len + i)).dim(),
						segment
					);
					if i != self.segments.len() - common_prefix_len - 1 {
						println!();
					}
				}
			}
		};
	}
}

impl fmt::Display for SimpleTuple {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.segments.is_empty() {
			write!(f, "/")?;
		} else {
			for segment in &self.segments {
				write!(f, "/{segment}")?;
			}
		}

		Ok(())
	}
}

impl TuplePack for SimpleTuple {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		for segment in &self.segments {
			offset += segment.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for SimpleTuple {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let mut input = input;
		let mut segments = Vec::new();

		loop {
			let (i, v) = SimpleTupleSegment::unpack(input, tuple_depth)?;
			input = i;
			segments.push(v);

			if input.is_empty() {
				break;
			}
		}

		let v = SimpleTuple { segments };

		Ok((input, v))
	}
}

fn unescape(s: &str) -> String {
	let mut result = String::new();
	let mut escaped = false;

	for c in s.chars() {
		if escaped {
			result.push(c);
			escaped = false;
		} else if c == '\\' {
			escaped = true;
		} else {
			result.push(c);
		}
	}

	result
}
