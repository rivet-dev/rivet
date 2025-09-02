use std::{fmt, result::Result::Ok, str::FromStr};

use anyhow::*;
use clap::ValueEnum;
use rivet_term::console::style;
use universaldb::tuple::{PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset};
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
	Id(rivet_util::Id),
	String(String),
	Nested(Vec<SimpleTupleValue>),
	Bytes(Vec<u8>),
	Unknown(Vec<u8>),
}

impl SimpleTupleValue {
	fn parse_str(value: &str) -> Self {
		Self::parse_str_inner(value, true, true)
	}

	fn parse_str_inner(value: &str, convert_keys: bool, nested: bool) -> Self {
		if let Ok(v) = value.parse::<u64>() {
			SimpleTupleValue::U64(v)
		} else if let Ok(v) = value.parse::<i64>() {
			SimpleTupleValue::I64(v)
		} else if let Ok(v) = value.parse::<f64>() {
			SimpleTupleValue::F64(v)
		} else if let Ok(v) = Uuid::from_str(value) {
			SimpleTupleValue::Uuid(v)
		} else if let Ok(v) = rivet_util::Id::from_str(value) {
			SimpleTupleValue::Id(v)
		} else if let (true, Some(v)) = (convert_keys, udb_util::prelude::key_from_str(value)) {
			SimpleTupleValue::U64(v as u64)
		} else if nested && value.trim().starts_with('[') && value.trim().ends_with(']') {
			let mut items = Vec::new();
			let mut last = 1;
			let mut escaped = false;

			for (i, c) in value.chars().enumerate() {
				match c {
					'\\' if !escaped => escaped = true,
					',' if !escaped => {
						items.push(SimpleTupleValue::parse_str_inner(
							&value[last..i],
							false,
							true,
						));
						last = i + 1;
					}
					_ => escaped = false,
				}
			}

			items.push(SimpleTupleValue::parse_str_inner(
				&value[last..value.len() - 1],
				false,
				true,
			));

			SimpleTupleValue::Nested(items)
		} else {
			SimpleTupleValue::String(unescape(value))
		}
	}

	pub fn parse_str_with_type_hint(type_hint: Option<&str>, value: &str) -> Result<Self> {
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
					'[' | ']' if !escaped => break None,
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
			Some("id") => rivet_util::Id::from_str(value)
				.map(SimpleTupleValue::Id)
				.with_context(|| format!("Could not parse `{value}` as ID"))?,
			Some("json") => {
				let v = serde_json::from_str::<serde_json::Value>(value)
					.with_context(|| format!("Could not parse `{value}` as JSON"))?;
				let s = serde_json::to_string(&v)
					.with_context(|| format!("Could not parse `{value}` as JSON"))?;
				SimpleTupleValue::String(s)
			}
			Some("str") => SimpleTupleValue::String(value.to_string()),
			Some("bytes") | Some("b") => {
				let bytes = hex::decode(value.as_bytes())
					.with_context(|| format!("Could not parse `{value}` as hex encoded bytes"))?;
				SimpleTupleValue::Bytes(bytes)
			}
			Some(type_hint) => bail!("unknown type: `{type_hint}`"),
			_ => SimpleTupleValue::parse_str_inner(value, false, false),
		};

		Ok(parsed_value)
	}

	pub fn serialize(&self) -> Result<Vec<u8>> {
		let v = match self {
			SimpleTupleValue::U64(v) => v.to_be_bytes().to_vec(),
			SimpleTupleValue::I64(v) => v.to_be_bytes().to_vec(),
			SimpleTupleValue::F64(v) => v.to_be_bytes().to_vec(),
			SimpleTupleValue::Uuid(v) => v.as_bytes().to_vec(),
			SimpleTupleValue::Id(v) => v.as_bytes().to_vec(),
			SimpleTupleValue::String(v) => v.as_bytes().to_vec(),
			SimpleTupleValue::Nested(_) => todo!("unsupported"),
			SimpleTupleValue::Bytes(v) => v.clone(),
			SimpleTupleValue::Unknown(v) => v.clone(),
		};

		Ok(v)
	}

	pub fn deserialize(type_hint: Option<&str>, value: &[u8]) -> Result<Self> {
		let parsed_value = match type_hint {
			Some("u64") => SimpleTupleValue::U64(u64::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as u64"))?,
			)),
			Some("u32") => SimpleTupleValue::U64(u32::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as u32"))?,
			) as u64),
			Some("i32") => SimpleTupleValue::I64(i64::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as i64"))?,
			)),
			Some("i64") => SimpleTupleValue::I64(i32::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as i32"))?,
			) as i64),
			Some("f64") => SimpleTupleValue::F64(f64::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as f64"))?,
			)),
			Some("f32") => SimpleTupleValue::F64(f32::from_be_bytes(
				value
					.try_into()
					.with_context(|| format!("Could not parse `{value:?}` as f32"))?,
			) as f64),
			Some("uuid") => Uuid::from_slice(value)
				.map(SimpleTupleValue::Uuid)
				.with_context(|| format!("Could not parse `{value:?}` as UUID"))?,
			Some("id") => rivet_util::Id::from_slice(value)
				.map(SimpleTupleValue::Id)
				.with_context(|| format!("Could not parse `{value:?}` as ID"))?,
			Some("str") => std::str::from_utf8(value)
				.map(|x| x.to_string())
				.map(SimpleTupleValue::String)
				.with_context(|| format!("Could not parse `{value:?}` as string"))?,
			Some("bytes") | Some("b") => SimpleTupleValue::Bytes(value.to_vec()),
			Some(type_hint) => bail!("unknown type: `{type_hint}`"),
			_ => {
				if let Ok(value) = value.try_into() {
					SimpleTupleValue::U64(u64::from_be_bytes(value))
				} else if let Ok(value) = value.try_into() {
					SimpleTupleValue::U64(u32::from_be_bytes(value) as u64)
				} else if let Ok(v) = Uuid::from_slice(value) {
					SimpleTupleValue::Uuid(v)
				} else if let Ok(v) = rivet_util::Id::from_slice(value) {
					SimpleTupleValue::Id(v)
				} else if let Ok(v) = std::str::from_utf8(value) {
					SimpleTupleValue::String(v.to_string())
				} else {
					SimpleTupleValue::Bytes(value.to_vec())
				}
			}
		};

		Ok(parsed_value)
	}

	pub fn write(&self, f: &mut impl std::fmt::Write, convert_keys: bool) -> fmt::Result {
		match &self {
			SimpleTupleValue::U64(v) => {
				if let Ok(v) = (*v).try_into() {
					if let (true, Some(key)) = (convert_keys, udb_util::prelude::str_from_key(v)) {
						write!(
							f,
							"{} {}",
							style(key).green(),
							style(format!("({v})")).magenta().dim()
						)
					} else {
						write!(f, "{}", style(v).magenta())
					}
				} else {
					write!(f, "{}", style(v).magenta())
				}
			}
			SimpleTupleValue::I64(v) => write!(f, "{}", style(v).cyan()),
			SimpleTupleValue::F64(v) => write!(f, "{}", style(v).red()),
			SimpleTupleValue::Uuid(v) => write!(f, "{}", style(v).blue()),
			SimpleTupleValue::Id(v) => write!(f, "{}", style(v).blue()),
			SimpleTupleValue::String(v) => {
				if v.is_empty() {
					write!(f, "{}", style("<empty string>").dim())
				} else {
					if let Ok(json) = serde_json::from_str::<serde_json::Value>(&v)
						.map_err(Into::<anyhow::Error>::into)
						.and_then(|v| colored_json(&v))
					{
						write!(f, "{json}")
					} else {
						write!(f, "{}", style(v).green())
					}
				}
			}
			SimpleTupleValue::Nested(items) => {
				if items.is_empty() {
					write!(f, "{}", style("<empty nested>").dim())
				} else {
					write!(f, "{}", style("[").bold())?;

					let mut iter = items.iter();

					if let Some(item) = iter.next() {
						item.write(f, false)?;
					}

					for item in iter {
						write!(f, ", ")?;
						item.write(f, false)?;
					}

					write!(f, "{}", style("]").bold())
				}
			}
			SimpleTupleValue::Bytes(v) => {
				if v.is_empty() {
					write!(f, "{}", style("<empty bytes>").dim())?;
				} else {
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
				}

				Ok(())
			}
			SimpleTupleValue::Unknown(v) => {
				let hex_string = if v.len() > 512 { &v[..512] } else { v }
					.iter()
					.map(|byte| format!("{:02x}", byte))
					.collect::<String>();
				write!(f, "{}", style(hex_string).red().dim().italic())?;

				if v.len() > 512 {
					write!(
						f,
						"{} {}",
						style("...").red().dim().italic(),
						style(format!("({} bytes)", v.len())).dim()
					)?;
				}

				Ok(())
			}
		}
	}
}

impl fmt::Display for SimpleTupleValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.write(f, true)
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
			SimpleTupleValue::Id(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::String(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::Nested(v) => v.as_slice().pack(w, tuple_depth.increment()),
			SimpleTupleValue::Bytes(v) => v.pack(w, tuple_depth),
			SimpleTupleValue::Unknown(v) => {
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
		if let Ok((input, v)) = <u64>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::U64(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <i64>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::I64(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <f64>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::F64(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <Uuid>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::Uuid(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <rivet_util::Id>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::Id(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <String>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::String(v);
			Ok((input, v))
		} else if let Ok((input, v)) =
			<Vec<SimpleTupleValue>>::unpack(input, tuple_depth.increment())
		{
			let v = SimpleTupleValue::Nested(v);
			Ok((input, v))
		} else if let Ok((input, v)) = <Vec<u8>>::unpack(input, tuple_depth) {
			let v = SimpleTupleValue::Bytes(v);
			Ok((input, v))
		} else {
			let v = SimpleTupleValue::Unknown(input.to_vec());
			Ok((&input[0..0], v))
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleTupleSegment {
	value: SimpleTupleValue,
}

impl SimpleTupleSegment {
	fn parse_str(prefix: Option<&str>, value: &str) -> Result<Self> {
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
			Some("id") => rivet_util::Id::from_str(value)
				.map(SimpleTupleValue::Id)
				.with_context(|| format!("Could not parse `{value}` as ID"))?,
			Some("str") => SimpleTupleValue::String(value.to_string()),
			Some("nested") => {
				if !value.trim().starts_with('[') && !value.trim().ends_with(']') {
					bail!("nested segment must start and end with [ ]");
				}

				let mut items = Vec::new();
				let mut last = 0;
				let mut prefix = None;
				let mut escaped = false;

				for (i, c) in value.chars().enumerate() {
					match c {
						'\\' if !escaped => escaped = true,
						',' if !escaped => {
							items.push(
								SimpleTupleSegment::parse_str(prefix.take(), &value[last..i])?
									.value,
							);
							last = i;
						}
						':' if !escaped => {
							prefix = Some(&value[last..i]);
							last = i;
						}
						_ => escaped = false,
					}
				}

				items.push(SimpleTupleSegment::parse_str(prefix.take(), &value[last..])?.value);

				SimpleTupleValue::Nested(items)
			}
			Some("bytes") | Some("b") => {
				let bytes = hex::decode(value.as_bytes())
					.with_context(|| format!("Could not parse `{value}` as hex encoded bytes"))?;
				SimpleTupleValue::Bytes(bytes)
			}
			Some(prefix) => bail!("unknown type: `{prefix}`"),
			_ => SimpleTupleValue::parse_str(value),
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
		let mut depth = 0usize;

		for (i, c) in value.chars().enumerate() {
			match c {
				'/' => {
					if depth == 0 {
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
								segments.push(SimpleTupleSegment::parse_str(
									prefix.take(),
									segment.trim(),
								)?);
							}
						} else if start != 0 && i == start {
							segments.push(SimpleTupleSegment::parse_str(prefix.take(), "")?);
						}

						start = i + 1;
					}
				}
				'\\' if !escaped => escaped = true,
				':' if !escaped && prefix.is_none() => {
					prefix = Some(&value[start..i]);
					start = i + 1;
				}
				'[' if !escaped => {
					depth += 1;
				}
				']' if !escaped => {
					depth = depth.saturating_sub(1);
				}
				_ => escaped = false,
			}
		}

		let segment = value[start..].trim();
		if segment == ".." {
			if normal_segment_encountered {
				bail!("Invalid path: '..' cannot go after other segments");
			}

			back_count += 1;
		} else if segment == "." {
			// Noop
		} else if !segment.is_empty() {
			segments.push(SimpleTupleSegment::parse_str(
				prefix.take(),
				segment.trim(),
			)?);
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
