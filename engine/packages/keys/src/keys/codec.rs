use anyhow::{Result, ensure};
use bytes::Bytes;
use foundationdb_tuple::{self as tuple, TupleDepth, TupleUnpack};
use object_store::path::Path;
use percent_encoding::percent_decode_str;
use rivet_util_id::Id;
use std::{cell::RefCell, fmt::Write as _, sync::LazyLock};
use vbare::Versioned;

/// Converts a typed key to and from its address-space encoding.
pub trait KeyAddress: Sized {
	type Packed;
	type Input: ?Sized;

	fn pack(&self) -> Self::Packed;

	fn unpack(input: &Self::Input) -> Result<Self>;
}

/// The range of tuple-encoded keys that extend this key, as FoundationDB's `Subspace::range`
/// defines it. The range runs from `packed + 0x00` to `packed + 0xff` and excludes the packed key
/// itself. No tuple element begins with `0xff`, so the range holds every key with one or more
/// further components and nothing else. Only keys that end on a complete component implement it.
pub trait SubspaceRange {
	fn range(&self) -> std::ops::Range<Vec<u8>>;
}

/// A typed byte prefix passed to SlateDB's `scan_prefix(prefix, ..)`.
pub trait PrefixAddress {
	fn pack(&self) -> Bytes;
}

pub trait ValueCodec {
	/// What `decode` returns.
	type Value;

	/// What `encode` takes: the value itself, or for a codec whose version the writer selects
	/// at runtime, the value together with that version.
	type Input;

	fn encode(value: Self::Input) -> Result<Bytes>;

	fn decode(bytes: Bytes) -> Result<Self::Value>;
}

pub trait FormalKey: KeyAddress {
	type Codec: ValueCodec;

	fn serialize(&self, value: <Self::Codec as ValueCodec>::Input) -> Result<Bytes> {
		Self::Codec::encode(value)
	}

	fn deserialize(&self, bytes: Bytes) -> Result<<Self::Codec as ValueCodec>::Value> {
		Self::Codec::decode(bytes)
	}
}

pub struct VbareValue<W, const VERSION: u16>(std::marker::PhantomData<W>);

impl<W: Versioned, const VERSION: u16> ValueCodec for VbareValue<W, VERSION> {
	type Value = W::Latest;
	type Input = W::Latest;

	fn encode(value: Self::Value) -> Result<Bytes> {
		Ok(W::seal_to_bytes(&value, VERSION)?)
	}

	/// Shares `str` and `data` fields with SlateDB's buffer instead of copying them.
	fn decode(bytes: Bytes) -> Result<Self::Value> {
		Ok(W::decode(bytes)?)
	}
}

/// A vbare value of a family whose readers advance in a rolling deploy, so it has no compiled
/// write version: every writer encodes it at the version it selected, normally from the
/// topology's read floor, and a write that names no version does not compile.
pub struct SelectedVbareValue<W>(std::marker::PhantomData<W>);

/// A value and the version its writer selected for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtVersion<T> {
	pub value: T,
	pub version: u16,
}

impl<T> AtVersion<T> {
	pub fn new(value: T, version: u16) -> Self {
		Self { value, version }
	}
}

impl<W: Versioned> ValueCodec for SelectedVbareValue<W> {
	type Value = W::Latest;
	type Input = AtVersion<W::Latest>;

	fn encode(input: Self::Input) -> Result<Bytes> {
		Ok(W::seal_to_bytes(&input.value, input.version)?)
	}

	/// Shares `str` and `data` fields with SlateDB's buffer instead of copying them.
	fn decode(bytes: Bytes) -> Result<Self::Value> {
		Ok(W::decode(bytes)?)
	}
}

pub struct BytesValue;

impl ValueCodec for BytesValue {
	type Value = Bytes;
	type Input = Bytes;

	fn encode(value: Bytes) -> Result<Bytes> {
		Ok(value)
	}

	fn decode(bytes: Bytes) -> Result<Bytes> {
		Ok(bytes)
	}
}

pub struct UnitValue;

impl ValueCodec for UnitValue {
	type Value = ();
	type Input = ();

	fn encode(_: ()) -> Result<Bytes> {
		Ok(Bytes::new())
	}

	fn decode(bytes: Bytes) -> Result<()> {
		ensure!(bytes.is_empty(), "unit value must be empty");
		Ok(())
	}
}

pub trait TextComponent: Sized {
	fn to_text(&self) -> String;

	/// Appends the same text as `to_text` to `out`. Components on hot paths override it to
	/// write in place, so packing a key needs no buffer per component.
	fn write_text(&self, out: &mut String) {
		out.push_str(&self.to_text());
	}

	fn from_text(text: &str) -> Result<Self>;
}

impl TextComponent for u16 {
	fn to_text(&self) -> String {
		self.to_string()
	}

	fn write_text(&self, out: &mut String) {
		write_display(out, self);
	}

	fn from_text(text: &str) -> Result<Self> {
		let value: Self = text.parse()?;
		ensure!(value.to_text() == text, "noncanonical integer component");
		Ok(value)
	}
}

impl TextComponent for u64 {
	fn to_text(&self) -> String {
		self.to_string()
	}

	fn write_text(&self, out: &mut String) {
		write_display(out, self);
	}

	fn from_text(text: &str) -> Result<Self> {
		let value: Self = text.parse()?;
		ensure!(value.to_text() == text, "noncanonical integer component");
		Ok(value)
	}
}

impl TextComponent for String {
	fn to_text(&self) -> String {
		self.clone()
	}

	fn write_text(&self, out: &mut String) {
		out.push_str(self);
	}

	fn from_text(text: &str) -> Result<Self> {
		ensure!(!text.is_empty(), "empty text component");
		Ok(text.to_owned())
	}
}

impl TextComponent for uuid::Uuid {
	fn to_text(&self) -> String {
		self.to_string()
	}

	fn write_text(&self, out: &mut String) {
		write_display(out, self);
	}

	fn from_text(text: &str) -> Result<Self> {
		let value = Self::parse_str(text)?;
		ensure!(value.to_text() == text, "noncanonical UUID component");
		Ok(value)
	}
}

impl TextComponent for Id {
	fn to_text(&self) -> String {
		self.to_string()
	}

	fn write_text(&self, out: &mut String) {
		write_display(out, self);
	}

	fn from_text(text: &str) -> Result<Self> {
		let value = Id::parse(text)?;
		ensure!(value.to_text() == text, "noncanonical ID component");
		Ok(value)
	}
}

/// Appends a value's `Display` text to `out` without an intermediate `String`.
pub fn write_display(out: &mut String, value: &impl std::fmt::Display) {
	// Writing to a `String` cannot fail.
	let _ = write!(out, "{value}");
}

pub fn s3_parts(path: &Path) -> Result<Vec<String>> {
	path.as_ref()
		.split('/')
		.map(|part| Ok(percent_decode_str(part).decode_utf8()?.into_owned()))
		.collect()
}

pub trait TextTarget {
	type Component: TextComponent;

	fn accept(&self, value: Self::Component) -> Result<()>;
}

impl<T: TextComponent> TextTarget for RefCell<Option<T>> {
	type Component = T;

	fn accept(&self, value: T) -> Result<()> {
		self.replace(Some(value));
		Ok(())
	}
}

macro_rules! text_constant_target {
	($($ty:ty),* $(,)?) => {$ (
		impl TextTarget for $ty {
			type Component = $ty;
			fn accept(&self, value: Self::Component) -> Result<()> {
				ensure!(self == &value, "wrong typed key constant");
				Ok(())
			}
		}
	)*};
}

text_constant_target!(u16, u64, String, Id);

pub fn read_text_component<T: TextTarget>(target: &T, text: &str) -> Result<()> {
	target.accept(T::Component::from_text(text)?)
}

pub trait BinaryComponent: Sized {
	fn pack_into(&self, out: &mut Vec<u8>);

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self>;
}

pub trait PrefixComponent {
	fn pack_prefix_into(&self, out: &mut Vec<u8>);
}

impl<T: BinaryComponent> PrefixComponent for T {
	fn pack_prefix_into(&self, out: &mut Vec<u8>) {
		self.pack_into(out);
	}
}

/// A prefix inside a tuple byte-string component, including escaped zero bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytePrefix(Bytes);

impl BytePrefix {
	pub fn new(bytes: impl Into<Bytes>) -> Self {
		Self(bytes.into())
	}
}

impl PrefixComponent for BytePrefix {
	fn pack_prefix_into(&self, out: &mut Vec<u8>) {
		tuple::pack_into(&tuple::Bytes::from(self.0.as_ref()), out);
		let terminator = out.pop();
		assert_eq!(
			terminator,
			Some(0),
			"tuple byte prefix must end in a terminator"
		);
	}
}

pub trait BinaryTarget {
	type Component: BinaryComponent;

	fn accept(&self, value: Self::Component) -> tuple::PackResult<()>;
}

impl<T: BinaryComponent> BinaryTarget for RefCell<Option<T>> {
	type Component = T;

	fn accept(&self, value: T) -> tuple::PackResult<()> {
		self.replace(Some(value));
		Ok(())
	}
}

macro_rules! binary_constant_target {
	($($ty:ty),* $(,)?) => {$ (
		impl BinaryTarget for $ty {
			type Component = $ty;
			fn accept(&self, value: Self::Component) -> tuple::PackResult<()> {
				if self != &value {
					return Err(tuple::PackError::Message("wrong typed key constant".into()));
				}
				Ok(())
			}
		}
	)*};
}

binary_constant_target!(u16, u64, i64, String, Id, VolumeKey, OrderedI64);

pub fn read_binary_component<T: BinaryTarget>(
	target: &T,
	input: &mut &[u8],
	depth: TupleDepth,
) -> tuple::PackResult<()> {
	target.accept(T::Component::unpack_from(input, depth)?)
}

macro_rules! tuple_component {
	($($ty:ty),* $(,)?) => {$ (
		impl BinaryComponent for $ty {
			fn pack_into(&self, out: &mut Vec<u8>) {
				tuple::pack_into(self, out);
			}
			fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
				let (rest, value) = <$ty as TupleUnpack>::unpack(input, depth)?;
				*input = rest;
				Ok(value)
			}
		}
	)*};
}

tuple_component!(u16, u64, i64, String, uuid::Uuid);

impl BinaryComponent for [u8; 16] {
	fn pack_into(&self, out: &mut Vec<u8>) {
		tuple::pack_into(&tuple::Bytes::from(self.as_slice()), out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		let (rest, value) = <tuple::Bytes<'_> as TupleUnpack>::unpack(input, depth)?;
		let bytes = value
			.as_ref()
			.try_into()
			.map_err(|_| tuple::PackError::Message("expected 16-byte hash".into()))?;
		*input = rest;
		Ok(bytes)
	}
}

impl BinaryComponent for Option<Vec<u8>> {
	fn pack_into(&self, out: &mut Vec<u8>) {
		tuple::pack_into(&self.as_deref().map(tuple::Bytes::from), out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		let (rest, value) = <Option<tuple::Bytes<'_>> as TupleUnpack>::unpack(input, depth)?;
		*input = rest;
		Ok(value.map(|bytes| bytes.as_ref().to_vec()))
	}
}

impl BinaryComponent for Id {
	fn pack_into(&self, out: &mut Vec<u8>) {
		let Id::V1(data) = self;
		let mut bytes = [0u8; 19];
		bytes[0] = 1;
		bytes[1..].copy_from_slice(data);
		tuple::pack_into(&tuple::Bytes::from(bytes.as_slice()), out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		let (rest, value) = <tuple::Bytes<'_> as TupleUnpack>::unpack(input, depth)?;
		*input = rest;
		Id::from_slice(value.as_ref())
			.map_err(|error| tuple::PackError::Message(error.to_string().into()))
	}
}

pub use rivet_data::generated::shared_depot_types_v1::VolumeKey;

impl KeyAddress for VolumeKey {
	type Packed = Bytes;
	type Input = [u8];

	fn pack(&self) -> Bytes {
		let mut out = Vec::new();
		tuple::pack_into(&tuple::Bytes::from(&self.partition_key[..]), &mut out);
		if let Some(name) = &self.name {
			tuple::pack_into(&tuple::Bytes::from(&name[..]), &mut out);
		}
		out.into()
	}

	fn unpack(input: &[u8]) -> Result<Self> {
		let key = if let Ok((partition,)) = tuple::unpack::<(tuple::Bytes<'_>,)>(input) {
			Self::unnamed(partition.as_ref().to_vec())
		} else {
			let (partition, name): (tuple::Bytes<'_>, tuple::Bytes<'_>) = tuple::unpack(input)?;
			Self::named(partition.as_ref().to_vec(), name.as_ref().to_vec())
		};
		ensure!(key.pack().as_ref() == input, "noncanonical volume key");
		Ok(key)
	}
}

impl BinaryComponent for VolumeKey {
	fn pack_into(&self, out: &mut Vec<u8>) {
		tuple::pack_into(&tuple::Bytes::from(self.pack().as_ref()), out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		let (rest, value) = <tuple::Bytes<'_> as TupleUnpack>::unpack(input, depth)?;
		*input = rest;
		Self::unpack(value.as_ref())
			.map_err(|error| tuple::PackError::Message(error.to_string().into()))
	}
}

impl BinaryComponent for Bytes {
	fn pack_into(&self, out: &mut Vec<u8>) {
		tuple::pack_into(&tuple::Bytes::from(self.as_ref()), out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		let (rest, value) = <tuple::Bytes<'_> as TupleUnpack>::unpack(input, depth)?;
		*input = rest;
		Ok(Bytes::copy_from_slice(value.as_ref()))
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderedI64(pub i64);

/// A fixed-width, big-endian complemented sequence. Higher sequences sort first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescendingU64(pub u64);

impl BinaryComponent for DescendingU64 {
	fn pack_into(&self, out: &mut Vec<u8>) {
		let bytes = (u64::MAX - self.0).to_be_bytes();
		tuple::pack_into(&tuple::Bytes::from(bytes.as_slice()), out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		let (rest, value) = <tuple::Bytes<'_> as TupleUnpack>::unpack(input, depth)?;
		let bytes = value
			.as_ref()
			.try_into()
			.map_err(|_| tuple::PackError::Message("expected 8-byte descending sequence".into()))?;
		*input = rest;
		Ok(Self(u64::MAX - u64::from_be_bytes(bytes)))
	}
}

impl BinaryComponent for OrderedI64 {
	fn pack_into(&self, out: &mut Vec<u8>) {
		self.0.pack_into(out);
	}

	fn unpack_from<'a>(input: &mut &'a [u8], depth: TupleDepth) -> tuple::PackResult<Self> {
		Ok(Self(i64::unpack_from(input, depth)?))
	}
}

/// The first encoded byte of a tuple component with a fixed type marker.
pub trait PrefixStart {
	fn pack_start(out: &mut Vec<u8>);
}

impl PrefixStart for String {
	fn pack_start(out: &mut Vec<u8>) {
		tuple::pack_into(&"", out);
		assert_eq!(out.pop(), Some(0), "tuple string must end in a terminator");
	}
}

impl PrefixStart for Bytes {
	fn pack_start(out: &mut Vec<u8>) {
		tuple::pack_into(&tuple::Bytes::from(&[][..]), out);
		assert_eq!(out.pop(), Some(0), "tuple bytes must end in a terminator");
	}
}

impl PrefixStart for VolumeKey {
	fn pack_start(out: &mut Vec<u8>) {
		Bytes::pack_start(out);
	}
}

pub fn expect_tag(input: &mut &[u8], depth: TupleDepth, tag: usize) -> tuple::PackResult<()> {
	let found = u64::unpack_from(input, depth)?;
	if found != tag as u64 {
		return Err(tuple::PackError::Message(
			format!("unexpected key tag: expected {tag}, found {found}").into(),
		));
	}
	Ok(())
}

/// A placement hash: where Depot routes a volume, in place of the hash of its partition key.
pub type Placement = [u8; 16];

/// Packs the prefix of a placed key: the `PLACED` tag and the placement hash. A placed key
/// carries its placement in front of its own components, so a router reads it from the key
/// with no lookup, and keys that share a prefix of a placement land together.
///
/// Depot stores a volume's record by its placement hash and name only, as it does an unplaced
/// volume's 128-bit key hash, so two volumes with one name must never share a placement:
/// group related volumes under a shared prefix and give each a distinct suffix.
pub fn pack_placement(placement: &Placement, out: &mut Vec<u8>) {
	BinaryComponent::pack_into(&(crate::keys::tags::partition::PLACED as u64), out);
	placement.pack_into(out);
}

/// Reads the placement a placed key starts with, or `None` for any other key. Every volume's
/// route goes through this, so it reads the tuple bytes directly: the `PLACED` tag, then a
/// byte-string component of exactly 16 bytes, with zero bytes escaped as `00 ff`.
pub fn embedded_placement(packed: &[u8]) -> Option<Placement> {
	let rest = packed.strip_prefix(PLACED_TAG.as_slice())?;
	let mut bytes = rest.strip_prefix(&[BYTES_CODE])?.iter();
	let mut placement = [0u8; 16];
	let mut len = 0;
	loop {
		let byte = *bytes.next()?;
		let value = match byte {
			0x00 if bytes.as_slice().first() == Some(&0xff) => {
				bytes.next();
				0x00
			}
			0x00 => break,
			byte => byte,
		};
		*placement.get_mut(len)? = value;
		len += 1;
	}
	match len {
		16 => Some(placement),
		_ => None,
	}
}

/// The packed tag that starts every placed partition key. Routing checks it on every log
/// request, so it is packed once.
static PLACED_TAG: LazyLock<Vec<u8>> = LazyLock::new(|| {
	let mut tag = Vec::with_capacity(2);
	BinaryComponent::pack_into(&(crate::keys::tags::partition::PLACED as u64), &mut tag);
	tag
});

/// The initial buffer for packing one binary key. Keys are a few tags, a placement, a volume
/// name and some integers, which fit well under this.
const PACK_CAPACITY: usize = 128;

/// Packs a binary key into one presized buffer and returns it at its exact size. Packing into
/// an empty `Vec` reallocates several times per key, and `Bytes::from` a `Vec` with spare
/// capacity allocates a shared header and keeps the spare capacity alive for the key's life.
#[doc(hidden)]
pub fn pack_bytes(pack: impl FnOnce(&mut Vec<u8>)) -> bytes::Bytes {
	let mut out = Vec::with_capacity(PACK_CAPACITY);
	pack(&mut out);
	bytes::Bytes::from(out.into_boxed_slice())
}

/// The tuple type code of a byte string.
const BYTES_CODE: u8 = 0x01;

/// Skips the placement prefix of a placed key while unpacking it. The caller's canonical check
/// confirms the placement matches the one its components derive.
pub fn skip_placement(input: &mut &[u8], depth: TupleDepth) -> tuple::PackResult<()> {
	expect_tag(input, depth, crate::keys::tags::partition::PLACED)?;
	Placement::unpack_from(input, depth)?;
	Ok(())
}
