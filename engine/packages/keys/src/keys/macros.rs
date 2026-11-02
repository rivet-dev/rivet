#[macro_export]
#[doc(hidden)]
macro_rules! __key_struct {
	($vis:vis $name:ident ($($field_vis:vis $field:ident: $ty:ty),* $(,)?) {$($extra:tt)*}) => {
		#[derive(Clone, Debug, PartialEq, Eq)]
		$vis struct $name {
			$($field_vis $field: $ty,)*
			$($extra)*
		}
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __push_text_part {
	($parts:expr, $key:expr, {$field:ident}) => {
		$parts.push(($field).to_text());
	};
	($parts:expr, $key:expr, $tag:ident) => {
		$parts.push($crate::keys::tags::s3::$tag.to_owned());
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __read_text_part {
	($parts:expr, {$field:ident}) => {
		$crate::keys::codec::read_text_component(
			&$field,
			&$parts
				.next()
				.ok_or_else(|| ::anyhow::anyhow!("missing key component"))?,
		)?;
	};
	($parts:expr, $tag:ident) => {
		::anyhow::ensure!(
			$parts.next().as_deref() == Some($crate::keys::tags::s3::$tag),
			"wrong key tag"
		);
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __push_binary_part {
	($out:expr, $key:expr, $space:ident, {$field:ident}) => {
		($field).pack_into($out);
	};
	($out:expr, $key:expr, $space:ident, $tag:ident) => {
		$crate::keys::codec::BinaryComponent::pack_into(
			&($crate::keys::tags::$space::$tag as u64),
			$out,
		);
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __push_prefix_part {
	($out:expr, {$field:ident}) => {
		$crate::keys::codec::PrefixComponent::pack_prefix_into($field, $out);
	};
	($out:expr, $tag:ident) => {
		$crate::keys::codec::BinaryComponent::pack_into(
			&($crate::keys::tags::slatedb::$tag as u64),
			$out,
		);
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __read_binary_part {
	($input:expr, $depth:expr, $space:ident, {$field:ident}) => {
		$crate::keys::codec::read_binary_component(&$field, $input, $depth)?;
	};
	($input:expr, $depth:expr, $space:ident, $tag:ident) => {
		$crate::keys::codec::expect_tag($input, $depth, $crate::keys::tags::$space::$tag)?;
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __binary_key_impl {
	($name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?]; $space:ident) => {
		impl $name {
			pub fn pack_into(&self, out: &mut Vec<u8>) {
				#[allow(unused_imports)]
				use $crate::keys::codec::BinaryComponent as _;
				$(let $field = &self.$field;)*
				$( $crate::__push_binary_part!(out, self, $space, $part); )*
			}
		}
		impl $crate::keys::codec::KeyAddress for $name {
			type Packed = ::bytes::Bytes;
			type Input = [u8];

			fn pack(&self) -> Self::Packed {
				$crate::keys::codec::pack_bytes(|out| self.pack_into(out))
			}
			fn unpack(input: &[u8]) -> ::anyhow::Result<Self> {
				let key: Self = ::foundationdb_tuple::unpack(input)?;
				::anyhow::ensure!(key.pack().as_ref() == input, "noncanonical binary key");
				Ok(key)
			}
		}
		impl $crate::keys::codec::SubspaceRange for $name {
			fn range(&self) -> ::std::ops::Range<Vec<u8>> {
				let mut start = Vec::new();
				self.pack_into(&mut start);
				let mut end = start.clone();
				start.push(0x00);
				end.push(0xff);
				start..end
			}
		}
		impl<'de> ::foundationdb_tuple::TupleUnpack<'de> for $name {
			fn unpack(
				input: &'de [u8],
				depth: ::foundationdb_tuple::TupleDepth,
			) -> ::foundationdb_tuple::PackResult<(&'de [u8], Self)> {
				let mut remaining = input;
				$(let $field = std::cell::RefCell::new(None::<$ty>);)*
				$( $crate::__read_binary_part!(&mut remaining, depth, $space, $part); )*
				$(let $field = $field.into_inner().ok_or(::foundationdb_tuple::PackError::MissingBytes)?;)*
				Ok((remaining, Self::new($($field),*)))
			}
		}
	};
}

/// Like `__binary_key_impl`, for a key placed by `$placement(&key)`: its packed form starts
/// with the `PLACED` tag and that placement, then its own components.
#[macro_export]
#[doc(hidden)]
macro_rules! __placed_binary_key_impl {
	($name:ident ($($field:ident: $ty:ty),* $(,)?) placed_by($placement:path) => [$($part:tt),* $(,)?]; $space:ident) => {
		impl $name {
			pub fn pack_into(&self, out: &mut Vec<u8>) {
				#[allow(unused_imports)]
				use $crate::keys::codec::BinaryComponent as _;
				$crate::keys::codec::pack_placement(&$placement(self), out);
				$(let $field = &self.$field;)*
				$( $crate::__push_binary_part!(out, self, $space, $part); )*
			}

			/// Where Depot places this key's volume.
			pub fn placement(&self) -> $crate::keys::codec::Placement {
				$placement(self)
			}
		}
		impl $crate::keys::codec::KeyAddress for $name {
			type Packed = ::bytes::Bytes;
			type Input = [u8];

			fn pack(&self) -> Self::Packed {
				$crate::keys::codec::pack_bytes(|out| self.pack_into(out))
			}
			fn unpack(input: &[u8]) -> ::anyhow::Result<Self> {
				let key: Self = ::foundationdb_tuple::unpack(input)?;
				// Repacking derives the placement again, so a key with a wrong one is refused.
				::anyhow::ensure!(key.pack().as_ref() == input, "noncanonical binary key");
				Ok(key)
			}
		}
		impl $crate::keys::codec::SubspaceRange for $name {
			fn range(&self) -> ::std::ops::Range<Vec<u8>> {
				let mut start = Vec::new();
				self.pack_into(&mut start);
				let mut end = start.clone();
				start.push(0x00);
				end.push(0xff);
				start..end
			}
		}
		impl<'de> ::foundationdb_tuple::TupleUnpack<'de> for $name {
			fn unpack(
				input: &'de [u8],
				depth: ::foundationdb_tuple::TupleDepth,
			) -> ::foundationdb_tuple::PackResult<(&'de [u8], Self)> {
				let mut remaining = input;
				$crate::keys::codec::skip_placement(&mut remaining, depth)?;
				$(let $field = std::cell::RefCell::new(None::<$ty>);)*
				$( $crate::__read_binary_part!(&mut remaining, depth, $space, $part); )*
				$(let $field = $field.into_inner().ok_or(::foundationdb_tuple::PackError::MissingBytes)?;)*
				Ok((remaining, Self::new($($field),*)))
			}
		}
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __formal_value {
	($name:ident, vbare($wrapper:path, $version:expr)) => {
		impl $crate::keys::codec::FormalKey for $name {
			type Codec = $crate::keys::codec::VbareValue<$wrapper, { $version }>;
		}
	};
	($name:ident, vbare_selected($wrapper:path)) => {
		impl $crate::keys::codec::FormalKey for $name {
			type Codec = $crate::keys::codec::SelectedVbareValue<$wrapper>;
		}
	};
	($name:ident, bytes) => {
		impl $crate::keys::codec::FormalKey for $name {
			type Codec = $crate::keys::codec::BytesValue;
		}
	};
	($name:ident, unit) => {
		impl $crate::keys::codec::FormalKey for $name {
			type Codec = $crate::keys::codec::UnitValue;
		}
	};
}

#[macro_export]
macro_rules! s3_key {
	($vis:vis $name:ident ($($field_vis:vis $field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?]; value: $kind:ident $(($($value_args:tt)*))?;) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		impl $crate::keys::codec::KeyAddress for $name {
			type Packed = ::object_store::path::Path;
			type Input = ::object_store::path::Path;

			fn pack(&self) -> Self::Packed {
				#[allow(unused_imports)]
				use $crate::keys::codec::TextComponent as _;
				$(let $field = &self.$field;)*
				let mut parts = Vec::<String>::new();
				$( $crate::__push_text_part!(parts, self, $part); )*
				::object_store::path::Path::from_iter(parts)
			}
			fn unpack(input: &Self::Input) -> ::anyhow::Result<Self> {
				let parts = $crate::keys::codec::s3_parts(input)?;
				let mut parts = parts.into_iter();
				$(let $field = std::cell::RefCell::new(None::<$ty>);)*
				$( $crate::__read_text_part!(parts, $part); )*
				::anyhow::ensure!(parts.next().is_none(), "trailing S3 path components");
				$(let $field = $field.into_inner().ok_or_else(|| ::anyhow::anyhow!("missing key field"))?;)*
				let key = Self::new($($field),*);
				::anyhow::ensure!(key.pack() == *input, "noncanonical S3 path");
				Ok(key)
			}
		}
		$crate::__formal_value!($name, $kind $(($($value_args)*))?);
	};
}

/// Constructs an exact-match UPS subject from typed tuple components.
#[macro_export]
macro_rules! ups_subject {
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?];) => {
		$crate::__key_struct!($vis $name ($($field: $ty),*) { subject: Vec<u8>, });

		impl $name {
			pub fn new($($field: $ty),*) -> Self {
				// Subjects are a few tags and an identifier, so one allocation holds them.
				let mut key = Self {
					$($field,)*
					subject: Vec::with_capacity(32),
				};

				#[allow(unused_imports)]
				use $crate::keys::codec::BinaryComponent as _;

				$(let $field = &key.$field;)*
				$( $crate::__push_binary_part!(&mut key.subject, key, ups, $part); )*

				key
			}
		}

		impl ::std::convert::AsRef<[u8]> for $name {
			fn as_ref(&self) -> &[u8] {
				&self.subject
			}
		}
	};
}

/// A formal Depot volume address. The parts form the partition key, which alone chooses
/// placement. An optional `name [TAG, ...]` clause names one of several volumes that share that
/// partition key: the name is packed from partition tags with the same tuple codec, so it is a
/// compile-checked formal key, never free text.
/// The volume address methods of a named formal volume key.
#[macro_export]
#[doc(hidden)]
macro_rules! __volume_key_named {
	($name:ident [$($name_tag:ident),+]) => {
		impl $name {
			/// The packed volume name shared by every volume of this kind.
			pub fn volume_name() -> Vec<u8> {
				let mut out = Vec::new();
				$( $crate::keys::codec::BinaryComponent::pack_into(
					&($crate::keys::tags::partition::$name_tag as u64),
					&mut out,
				); )+
				out
			}
			pub fn volume_key(&self) -> $crate::keys::VolumeKey {
				$crate::keys::VolumeKey::named(
					$crate::keys::KeyAddress::pack(self).to_vec(),
					Self::volume_name(),
				)
			}
			pub fn from_volume_key(key: &$crate::keys::VolumeKey) -> ::anyhow::Result<Self> {
				::anyhow::ensure!(
					key.name.as_deref() == Some(Self::volume_name().as_slice()),
					"this formal volume address has a different name"
				);
				<$name as $crate::keys::KeyAddress>::unpack(&key.partition_key)
			}
		}
	};
}

/// The volume address methods of an unnamed formal volume key.
#[macro_export]
#[doc(hidden)]
macro_rules! __volume_key_unnamed {
	($name:ident) => {
		impl $name {
			pub fn volume_key(&self) -> $crate::keys::VolumeKey {
				$crate::keys::VolumeKey::unnamed($crate::keys::KeyAddress::pack(self).to_vec())
			}
			pub fn from_volume_key(key: &$crate::keys::VolumeKey) -> ::anyhow::Result<Self> {
				::anyhow::ensure!(key.name.is_none(), "this formal volume address has no name");
				<$name as $crate::keys::KeyAddress>::unpack(&key.partition_key)
			}
		}
	};
}

#[macro_export]
macro_rules! volume_key {
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) placed_by($placement:path) => [$($part:tt),* $(,)?] name [$($name_tag:ident),+ $(,)?];) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__placed_binary_key_impl!($name ($($field: $ty),*) placed_by($placement) => [$($part),*]; partition);
		$crate::__volume_key_named!($name [$($name_tag),+]);
	};
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) placed_by($placement:path) => [$($part:tt),* $(,)?];) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__placed_binary_key_impl!($name ($($field: $ty),*) placed_by($placement) => [$($part),*]; partition);
		$crate::__volume_key_unnamed!($name);
	};
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?] name [$($name_tag:ident),+ $(,)?];) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__binary_key_impl!($name ($($field: $ty),*) => [$($part),*]; partition);
		impl $name {
			/// The packed volume name shared by every volume of this kind.
			pub fn volume_name() -> Vec<u8> {
				let mut out = Vec::new();
				$( $crate::keys::codec::BinaryComponent::pack_into(
					&($crate::keys::tags::partition::$name_tag as u64),
					&mut out,
				); )+
				out
			}
			pub fn volume_key(&self) -> $crate::keys::VolumeKey {
				$crate::keys::VolumeKey::named(
					$crate::keys::KeyAddress::pack(self).to_vec(),
					Self::volume_name(),
				)
			}
			pub fn from_volume_key(key: &$crate::keys::VolumeKey) -> ::anyhow::Result<Self> {
				::anyhow::ensure!(
					key.name.as_deref() == Some(Self::volume_name().as_slice()),
					"this formal volume address has a different name"
				);
				<$name as $crate::keys::KeyAddress>::unpack(&key.partition_key)
			}
		}
	};
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?];) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__binary_key_impl!($name ($($field: $ty),*) => [$($part),*]; partition);
		impl $name {
			pub fn volume_key(&self) -> $crate::keys::VolumeKey {
				$crate::keys::VolumeKey::unnamed($crate::keys::KeyAddress::pack(self).to_vec())
			}
			pub fn from_volume_key(key: &$crate::keys::VolumeKey) -> ::anyhow::Result<Self> {
				::anyhow::ensure!(key.name.is_none(), "this formal volume address has no name");
				<$name as $crate::keys::KeyAddress>::unpack(&key.partition_key)
			}
		}
	};
}

/// A typed application key inside a Depot volume.
#[macro_export]
macro_rules! volume_kv_key {
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) placed_by($placement:path) => [$($part:tt),* $(,)?]; value: $kind:ident $(($($value_args:tt)*))?;) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__placed_binary_key_impl!($name ($($field: $ty),*) placed_by($placement) => [$($part),*]; partition);
		$crate::__formal_value!($name, $kind $(($($value_args)*))?);
	};
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?]; value: $kind:ident $(($($value_args:tt)*))?;) => {
		$crate::volume_kv_key!($vis $name ($($field: $ty),*) => [$($part),*];);
		$crate::__formal_value!($name, $kind $(($($value_args)*))?);
	};
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?];) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__binary_key_impl!($name ($($field: $ty),*) => [$($part),*]; partition);
	};
}

#[macro_export]
macro_rules! slatedb_key {
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?]; value: $kind:ident $(($($value_args:tt)*))?;) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		$crate::__binary_key_impl!($name ($($field: $ty),*) => [$($part),*]; slatedb);
		$crate::__formal_value!($name, $kind $(($($value_args)*))?);
	};
}

/// Defines an address for SlateDB's raw byte-prefix scan, without a value codec.
/// Use `next` when the last complete component is a byte string or string and
/// the scan should include only rows with a following component of that type.
#[macro_export]
macro_rules! slatedb_prefix {
	($vis:vis $name:ident ($($field:ident: $ty:ty),* $(,)?) => [$($part:tt),* $(,)?]; $(next: $next:ty;)?) => {
		$crate::__key_struct!($vis $name ($(pub $field: $ty),*) {});
		impl $name {
			pub fn new($($field: $ty),*) -> Self { Self { $($field),* } }
		}
		impl $crate::keys::codec::PrefixAddress for $name {
			fn pack(&self) -> ::bytes::Bytes {
				$(let $field = &self.$field;)*
				$crate::keys::codec::pack_bytes(|out| {
					$( $crate::__push_prefix_part!(out, $part); )*
					$(<$next as $crate::keys::codec::PrefixStart>::pack_start(out);)?
				})
			}
		}
	};
}
