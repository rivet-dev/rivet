//! Canonical text codecs at the HTTP boundary; filesystem paths and keys remain bytes.
use super::{Error, Result};
use depot_types::VolumeKey;
use std::collections::BTreeMap;

pub(crate) struct Address {
	pub volume: VolumeKey,
	pub kind: Kind,
	pub path: Vec<u8>,
	pub query: BTreeMap<Vec<u8>, Vec<u8>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
	Stat,
	Dirs,
	Files,
	Kv,
}
impl Address {
	pub fn parse(uri: &http::Uri, max_path_bytes: usize) -> Result<Self> {
		if uri.path().len() > max_path_bytes.saturating_mul(3).saturating_add(16_384)
			|| uri.query().is_some_and(|q| q.len() > 16_384)
		{
			return Err(Error::TooLarge);
		}
		let mut parts = uri.path().splitn(6, '/');
		if parts.next() != Some("") || parts.next() != Some("volumes") {
			return Err(Error::NotFound);
		}
		let partition = parts.next().ok_or(Error::Invalid)?;
		let name = parts.next().ok_or(Error::Invalid)?;
		let volume = VolumeKey {
			partition_key: if partition == "-" {
				vec![].into()
			} else {
				(unhex(partition)?).into()
			},
			name: match name {
				"-" => None,
				"~" => Some(vec![].into()),
				name => Some((unhex(name)?).into()),
			},
		};
		if volume.byte_len() > 4096 {
			return Err(Error::TooLarge);
		}
		let kind = match parts.next() {
			Some("stat") => Kind::Stat,
			Some("dirs") => Kind::Dirs,
			Some("files") => Kind::Files,
			Some("kv") => Kind::Kv,
			_ => return Err(Error::NotFound),
		};
		let mut path = vec![b'/'];
		path.extend(decode(parts.next().unwrap_or("").as_bytes(), false)?);
		if path.len() > max_path_bytes || path.contains(&0) {
			return Err(Error::Invalid);
		}
		let mut query = BTreeMap::new();
		for pair in uri
			.query()
			.unwrap_or("")
			.split('&')
			.filter(|s| !s.is_empty())
		{
			let (name, value) = pair.split_once('=').ok_or(Error::Invalid)?;
			let name = decode(name.as_bytes(), true)?;
			let value = decode(value.as_bytes(), true)?;
			if !matches!(
				(kind, name.as_slice()),
				(Kind::Dirs, b"cursor") | (Kind::Kv, b"key")
			) || query.insert(name, value).is_some()
			{
				return Err(Error::Invalid);
			}
		}
		Ok(Self {
			volume,
			kind,
			path,
			query,
		})
	}
}
fn unhex(value: &str) -> Result<Vec<u8>> {
	// One canonical spelling for binary identifiers, including empty components.
	if value.is_empty()
		|| value
			.bytes()
			.any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
	{
		return Err(Error::Invalid);
	}
	hex::decode(value).map_err(|_| Error::Invalid)
}
fn decode(value: &[u8], query: bool) -> Result<Vec<u8>> {
	let mut result = Vec::with_capacity(value.len());
	let mut i = 0;
	while i < value.len() {
		match value[i] {
			b'%' => {
				let encoded = value.get(i + 1..i + 3).ok_or(Error::Invalid)?;
				let decoded = hex::decode(encoded).map_err(|_| Error::Invalid)?;
				result.push(decoded[0]);
				i += 3;
			}
			b'+' if query => {
				result.push(b' ');
				i += 1;
			}
			byte => {
				result.push(byte);
				i += 1;
			}
		}
	}
	Ok(result)
}

/// End is exclusive. A single range can be streamed without buffering the file.
pub(crate) fn range(value: Option<&str>, size: u64) -> Result<(u64, u64)> {
	let Some(value) = value else {
		return Ok((0, size));
	};
	let (start, end) = value
		.strip_prefix("bytes=")
		.and_then(|v| v.split_once('-'))
		.ok_or(Error::Range(size))?;
	let number = |s: &str| {
		if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
			return Err(Error::Range(size));
		}
		s.parse::<u64>().map_err(|_| Error::Range(size))
	};
	if size == 0 {
		return Err(Error::Range(size));
	}
	if start.is_empty() {
		let count = number(end)?;
		if count == 0 {
			return Err(Error::Range(size));
		}
		return Ok((size.saturating_sub(count), size));
	}
	let start = number(start)?;
	let end = if end.is_empty() {
		size
	} else {
		number(end)?.saturating_add(1).min(size)
	};
	if start >= end {
		return Err(Error::Range(size));
	}
	Ok((start, end))
}
