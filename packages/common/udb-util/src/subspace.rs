use std::{borrow::Cow, ops::Deref};

use rivet_metrics::KeyValue;
use universaldb::{
	KeySelector, RangeOption,
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

	/// Returns a new Subspace whose prefix extends this Subspace with a given tuple encodable.
	pub fn subspace<T: TuplePack>(&self, t: &T) -> Self {
		Self {
			inner: self.inner.subspace(t),
		}
	}

	/// Returns the key encoding the specified Tuple with the prefix of this Subspace
	/// prepended.
	pub fn pack<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		metrics::KEY_PACK_COUNT.add(1, &[KeyValue::new("type", std::any::type_name::<T>())]);

		self.inner.pack(t)
	}

	/// Returns the key encoding the specified Tuple with the prefix of this Subspace
	/// prepended, with a versionstamp.
	pub fn pack_with_versionstamp<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		metrics::KEY_PACK_COUNT.add(1, &[KeyValue::new("type", std::any::type_name::<T>())]);

		self.inner.pack_with_versionstamp(t)
	}

	/// `unpack` returns the Tuple encoded by the given key with the prefix of this Subspace
	/// removed.  `unpack` will return an error if the key is not in this Subspace or does not
	/// encode a well-formed Tuple.
	pub fn unpack<'de, T: TupleUnpack<'de>>(&self, key: &'de [u8]) -> PackResult<T> {
		metrics::KEY_UNPACK_COUNT.add(1, &[KeyValue::new("type", std::any::type_name::<T>())]);

		self.inner.unpack(key)
	}
}

impl Deref for Subspace {
	type Target = tuple::Subspace;

	fn deref(&self) -> &Self::Target {
		&self.inner
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
