use anyhow::{Context, Result};

use crate::{tuple::TupleUnpack, utils::FormalKey};

pub trait SliceExt {
	fn read<'de, T: FormalKey + TupleUnpack<'de>>(&self, key: &'de T) -> Result<T::Value>;
}

pub trait OptSliceExt {
	fn read<'de, T: FormalKey + TupleUnpack<'de>>(&self, key: &'de T) -> Result<T::Value>;
	fn read_opt<'de, T: FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<Option<T::Value>>;
}

impl SliceExt for crate::value::Slice {
	fn read<'de, T: FormalKey + TupleUnpack<'de>>(&self, key: &'de T) -> Result<T::Value> {
		key.deserialize(self).with_context(|| {
			format!(
				"failed deserializing key value of {}",
				std::any::type_name::<T>(),
			)
		})
	}
}

impl OptSliceExt for Option<crate::value::Slice> {
	fn read<'de, T: FormalKey + TupleUnpack<'de>>(&self, key: &'de T) -> Result<T::Value> {
		key.deserialize(
			&self
				.as_ref()
				.with_context(|| format!("key should exist: {}", std::any::type_name::<T>()))?,
		)
		.with_context(|| {
			format!(
				"failed deserializing key value of {}",
				std::any::type_name::<T>(),
			)
		})
	}

	fn read_opt<'de, T: FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<Option<T::Value>> {
		if let Some(data) = self {
			key.deserialize(data).map(Some).with_context(|| {
				format!(
					"failed deserializing key value of {}",
					std::any::type_name::<T>(),
				)
			})
		} else {
			Ok(None)
		}
	}
}
