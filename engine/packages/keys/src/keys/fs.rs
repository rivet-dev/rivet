//! Relative filesystem formal keys. Literal and typed components each occupy one
//! filename. Separators, percent signs, and dot-only names are escaped, so a field
//! cannot turn a key into an absolute path or traverse its configured root.

use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use super::codec::TextComponent;

const ESCAPES: &AsciiSet = &CONTROLS.add(b'/').add(b'\\').add(b'%').add(b':');

/// Room for a key's directories and filename. Longer keys grow the buffer.
const PACK_CAPACITY: usize = 64;

/// Builds a packed filesystem key in one presized buffer. Each component is written in place
/// and escaped only when its text needs it, so packing a key allocates once.
#[doc(hidden)]
pub struct KeyWriter {
	out: String,
	/// Length of the root the key is packed beneath, which takes no separator before it.
	start: usize,
}

impl KeyWriter {
	/// Starts a relative key.
	pub fn new() -> Self {
		Self {
			out: String::with_capacity(PACK_CAPACITY),
			start: 0,
		}
	}

	/// Starts a key beneath `root`, so the root and the key share one allocation. A root that
	/// is not UTF-8 cannot be written into the buffer and is joined by the caller instead.
	pub fn under(root: &Path) -> Option<Self> {
		let root = root.to_str()?;
		let mut out = String::with_capacity(root.len() + 1 + PACK_CAPACITY);
		out.push_str(root);
		if !root.is_empty() && !root.ends_with(std::path::MAIN_SEPARATOR) {
			out.push(std::path::MAIN_SEPARATOR);
		}
		let start = out.len();
		Some(Self { out, start })
	}

	pub fn literal(&mut self, part: &str) {
		self.separator();
		escape_into(&mut self.out, part);
	}

	pub fn text(&mut self, part: &impl TextComponent) {
		self.separator();
		let start = self.out.len();
		part.write_text(&mut self.out);
		if needs_escape(&self.out[start..]) {
			let raw = self.out.split_off(start);
			escape_into(&mut self.out, &raw);
		}
	}

	pub fn finish(self) -> PathBuf {
		PathBuf::from(self.out)
	}

	fn separator(&mut self) {
		if self.out.len() > self.start {
			self.out.push(std::path::MAIN_SEPARATOR);
		}
	}
}

impl Default for KeyWriter {
	fn default() -> Self {
		Self::new()
	}
}

/// Whether a component's text differs from its escaped spelling.
fn needs_escape(part: &str) -> bool {
	if matches!(part, "" | "." | "..") {
		return true;
	}
	!matches!(
		utf8_percent_encode(part, ESCAPES).next(),
		Some(chunk) if chunk.len() == part.len()
	)
}

/// Appends one component's escaped spelling. Empty and dot-only names are spelled so they
/// cannot collapse or traverse a directory.
fn escape_into(out: &mut String, part: &str) {
	match part {
		"" => out.push('%'),
		"." => out.push_str("%2E"),
		".." => out.push_str("%2E%2E"),
		_ => out.extend(utf8_percent_encode(part, ESCAPES)),
	}
}

#[doc(hidden)]
pub fn unpack_parts(path: &Path) -> Result<Vec<String>> {
	path.components()
		.map(|component| {
			let Component::Normal(component) = component else {
				bail!("filesystem key must contain only relative normal components");
			};
			let text = component
				.to_str()
				.ok_or_else(|| anyhow::anyhow!("non-UTF-8 filesystem key"))?;
			if text == "%" {
				return Ok(String::new());
			}
			Ok(percent_decode_str(text).decode_utf8()?.into_owned())
		})
		.collect()
}

#[macro_export]
#[doc(hidden)]
macro_rules! __push_fs_part {
	($writer:expr, {$field:ident}) => {
		$writer.text($field);
	};
	($writer:expr, $literal:literal) => {
		$writer.literal($literal);
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __read_fs_part {
	($parts:expr, {$field:ident}) => {
		$crate::keys::codec::read_text_component(
			&$field,
			&$parts
				.next()
				.ok_or_else(|| ::anyhow::anyhow!("missing filesystem component"))?,
		)?;
	};
	($parts:expr, $literal:literal) => {
		::anyhow::ensure!(
			$parts.next().as_deref() == Some($literal),
			"wrong filesystem component"
		);
	};
}

/// Defines a relative filesystem address and its value codec. Components are
/// string literals or `{field}` references implementing `TextComponent`.
/// `pack` returns a `PathBuf` to join beneath the caller's configured root;
/// `unpack` rejects absolute paths and noncanonical spellings.
#[macro_export]
macro_rules! fs_key {
	($vis:vis $name:ident ($($field_vis:vis $field:ident: $ty:ty),* $(,)?) => [$($part:tt),+ $(,)?]; value: $kind:ident $(($($value_args:tt)*))?;) => {
		$crate::__key_struct!($vis $name ($($field_vis $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }

			/// The key's path beneath `root`, the same as `root.join(self.pack())`.
			pub fn pack_under(&self, root: &::std::path::Path) -> ::std::path::PathBuf {
				match $crate::keys::fs::KeyWriter::under(root) {
					Some(mut writer) => {
						self.write_parts(&mut writer);
						writer.finish()
					}
					None => root.join($crate::keys::codec::KeyAddress::pack(self)),
				}
			}

			fn write_parts(&self, writer: &mut $crate::keys::fs::KeyWriter) {
				$(let $field = &self.$field;)*
				$($crate::__push_fs_part!(writer, $part);)*
			}
		}
		impl $crate::keys::codec::KeyAddress for $name {
			type Packed = ::std::path::PathBuf;
			type Input = ::std::path::Path;

			fn pack(&self) -> Self::Packed {
				let mut writer = $crate::keys::fs::KeyWriter::new();
				self.write_parts(&mut writer);
				writer.finish()
			}

			fn unpack(input: &Self::Input) -> ::anyhow::Result<Self> {
				let mut parts = $crate::keys::fs::unpack_parts(input)?.into_iter();
				$(let $field = ::std::cell::RefCell::new(None::<$ty>);)*
				$($crate::__read_fs_part!(parts, $part);)*
				::anyhow::ensure!(parts.next().is_none(), "trailing filesystem components");
				$(let $field = $field.into_inner().ok_or_else(|| ::anyhow::anyhow!("missing key field"))?;)*
				let key = Self::new($($field),*);
				::anyhow::ensure!(
					key.pack().as_os_str() == input.as_os_str(),
					"noncanonical filesystem key"
				);
				Ok(key)
			}
		}
		$crate::__formal_value!($name, $kind $(($($value_args)*))?);
	};
}
