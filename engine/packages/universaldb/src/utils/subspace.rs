use std::{borrow::Cow, ops::Deref};

use crate::{
	key_selector::KeySelector,
	range_option::RangeOption,
	tuple::{self, PackResult, TuplePack, TupleUnpack},
};

use crate::metrics;

/// Wrapper type around `universaldb::tuple::Subspace` that records metrics.
#[derive(Clone)]
pub struct Subspace {
	inner: tuple::Subspace,
}

impl Subspace {
	/// Creates a subspace with the given tuple.
	pub fn new<T: TuplePack>(t: &T) -> Self {
		Self {
			inner: tuple::Subspace::all().subspace(t),
		}
	}

	pub fn all() -> Self {
		Self {
			inner: tuple::Subspace::all(),
		}
	}

	/// Returns a new Subspace whose prefix extends this Subspace with a given tuple encodable.
	pub fn subspace<T: TuplePack>(&self, t: &T) -> Self {
		Self {
			inner: self.inner.subspace(t),
		}
	}

	pub fn join(&self, s: &Subspace) -> Self {
		Self {
			inner: tuple::Subspace::from_bytes([self.inner.bytes(), s.bytes()].concat()),
		}
	}

	/// Returns the key encoding the specified Tuple with the prefix of this Subspace
	/// prepended.
	pub fn pack<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		metrics::KEY_PACK_COUNT
			.with_label_values(&[std::any::type_name::<T>()])
			.inc();

		self.inner.pack(t)
	}

	/// Returns the key encoding the specified Tuple with the prefix of this Subspace
	/// prepended, with a versionstamp.
	pub fn pack_with_versionstamp<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		metrics::KEY_PACK_COUNT
			.with_label_values(&[std::any::type_name::<T>()])
			.inc();

		self.inner.pack_with_versionstamp(t)
	}

	/// `unpack` returns the Tuple encoded by the given key with the prefix of this Subspace
	/// removed.  `unpack` will return an error if the key is not in this Subspace or does not
	/// encode a well-formed Tuple.
	pub fn unpack<'de, T: TupleUnpack<'de>>(&self, key: &'de [u8]) -> PackResult<T> {
		metrics::KEY_UNPACK_COUNT
			.with_label_values(&[std::any::type_name::<T>()])
			.inc();

		self.inner.unpack(key)
	}
}

impl Deref for Subspace {
	type Target = tuple::Subspace;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl From<tuple::Subspace> for Subspace {
	fn from(value: tuple::Subspace) -> Self {
		Subspace { inner: value }
	}
}

impl<'a> From<&'a Subspace> for RangeOption<'static> {
	fn from(subspace: &Subspace) -> Self {
		let (begin, end) = subspace.range();

		Self {
			begin: KeySelector::first_greater_or_equal(Cow::Owned(begin)),
			end: KeySelector::first_greater_or_equal(Cow::Owned(end)),
			..Self::default()
		}
	}
}
