use std::{fmt::Debug, ops::Deref, result::Result::Ok};

use anyhow::*;
use futures_util::TryStreamExt;
use universaldb::{
	self as udb,
	options::{ConflictRangeType, MutationType, StreamingMode},
	tuple::{TuplePack, TupleUnpack},
};

use crate::{FormalKey, Subspace, end_of_key_range};

pub trait TxnExt {
	fn subspace<'a>(&'a self, subspace: Subspace) -> TxnSubspace<'a>;
}

impl TxnExt for udb::Transaction {
	fn subspace<'a>(&'a self, subspace: Subspace) -> TxnSubspace<'a> {
		TxnSubspace {
			tx: &self,
			subspace,
		}
	}
}

#[derive(Clone)]
pub struct TxnSubspace<'a> {
	tx: &'a udb::Transaction,
	subspace: Subspace,
}

impl<'a> TxnSubspace<'a> {
	pub fn subspace<T: TuplePack>(&self, t: &T) -> Subspace {
		self.subspace.subspace(t)
	}

	pub fn pack<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		self.subspace.pack(t)
	}

	pub fn unpack<'de, T: TupleUnpack<'de>>(
		&self,
		key: &'de [u8],
	) -> Result<T, udb::FdbBindingError> {
		self.subspace
			.unpack(key)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
	}

	pub fn write<T: FormalKey + TuplePack>(
		&self,
		key: &T,
		value: T::Value,
	) -> Result<(), udb::FdbBindingError> {
		self.tx.set(
			&self.subspace.pack(key),
			&key.serialize(value)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		Ok(())
	}

	pub async fn read<'de, T: Debug + FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		snapshot: bool,
	) -> Result<T::Value, udb::FdbBindingError> {
		self.tx
			.get(&self.subspace.pack(key), snapshot)
			.await?
			.read(key)
	}

	pub async fn read_opt<'de, T: FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		snapshot: bool,
	) -> Result<Option<T::Value>, udb::FdbBindingError> {
		self.tx
			.get(&self.subspace.pack(key), snapshot)
			.await?
			.read_opt(key)
	}

	pub async fn exists<T: TuplePack>(
		&self,
		key: &T,
		snapshot: bool,
	) -> Result<bool, udb::FdbBindingError> {
		Ok(self
			.tx
			.get(&self.subspace.pack(key), snapshot)
			.await?
			.is_some())
	}

	pub fn delete<T: TuplePack>(&self, key: &T) {
		self.tx.clear(&self.subspace.pack(key));
	}

	pub fn read_entry<T: FormalKey + for<'de> TupleUnpack<'de>>(
		&self,
		entry: &udb::future::FdbValue,
	) -> Result<(T, T::Value), udb::FdbBindingError> {
		let key = self.unpack::<T>(entry.key())?;
		let value = key
			.deserialize(entry.value())
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

		Ok((key, value))
	}

	pub async fn cherry_pick<T: CherryPick>(
		&self,
		subspace: impl TuplePack + Send,
		snapshot: bool,
	) -> Result<T::Output, udb::FdbBindingError> {
		T::cherry_pick(self, subspace, snapshot).await
	}

	pub fn add_conflict_key<T: TuplePack>(
		&self,
		key: &T,
		conflict_type: ConflictRangeType,
	) -> Result<(), udb::FdbBindingError> {
		let key_buf = self.subspace.pack(key);

		self.tx
			.add_conflict_range(&key_buf, &end_of_key_range(&key_buf), conflict_type)
			.map_err(Into::into)
	}

	pub fn atomic_op<'de, T: Debug + FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		param: &[u8],
		op_type: MutationType,
	) {
		self.tx.atomic_op(&self.subspace.pack(key), param, op_type)
	}
}

impl<'a> Deref for TxnSubspace<'a> {
	type Target = udb::Transaction;

	fn deref(&self) -> &Self::Target {
		self.tx
	}
}

pub trait SliceExt {
	fn read<'de, T: FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<T::Value, udb::FdbBindingError>;
}

pub trait OptSliceExt {
	fn read<'de, T: Debug + FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<T::Value, udb::FdbBindingError>;
	fn read_opt<'de, T: FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<Option<T::Value>, udb::FdbBindingError>;
}

impl SliceExt for udb::future::FdbSlice {
	fn read<'de, T: FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<T::Value, udb::FdbBindingError> {
		key.deserialize(self)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
	}
}

impl OptSliceExt for Option<udb::future::FdbSlice> {
	fn read<'de, T: Debug + FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<T::Value, udb::FdbBindingError> {
		key.deserialize(&self.as_ref().ok_or(udb::FdbBindingError::CustomError(
			format!("key should exist: {key:?}").into(),
		))?)
		.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
	}

	fn read_opt<'de, T: FormalKey + TupleUnpack<'de>>(
		&self,
		key: &'de T,
	) -> Result<Option<T::Value>, udb::FdbBindingError> {
		if let Some(data) = self {
			key.deserialize(data)
				.map(Some)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
		} else {
			Ok(None)
		}
	}
}

#[async_trait::async_trait]
pub trait CherryPick {
	type Output;

	async fn cherry_pick<S: TuplePack + Send>(
		txs: &TxnSubspace<'_>,
		subspace: S,
		snapshot: bool,
	) -> Result<Self::Output, udb::FdbBindingError>;
}

// Implements `CherryPick` for any tuple size
macro_rules! impl_tuple {
    ($($args:ident),*) => {
        #[async_trait::async_trait]
        impl<$($args: FormalKey + for<'de> TupleUnpack<'de>),*> CherryPick for ($($args),*)
        where
            $($args::Value: Send),*
        {
            type Output = ($($args::Value),*);

            async fn cherry_pick<S: TuplePack + Send>(
                txs: &TxnSubspace<'_>,
                subspace: S,
                snapshot: bool,
            ) -> Result<Self::Output, udb::FdbBindingError> {
				let subspace = txs.subspace(&subspace);

                let mut stream = txs.get_ranges_keyvalues(
                    udb::RangeOption {
                        mode: StreamingMode::WantAll,
                        ..(&subspace).into()
                    },
                    snapshot,
                );

                $(
					#[allow(non_snake_case)]
                    let mut $args = None;
                )*

                loop {
                    let Some(entry) = stream.try_next().await? else {
                        break;
                    };

                    $(
                        if let Ok(key) = txs.unpack::<$args>(entry.key()) {
                            if $args.is_some() {
                                return Err(udb::FdbBindingError::CustomError(
                                    format!("{} already picked", std::any::type_name::<$args>()).into()
                                ));
                            }

                            let value = key.read(entry.value())?;
                            $args = Some(value);
                            continue;
                        }
                    )*
                }

                Ok((
                    $(
                        $args.ok_or(udb::FdbBindingError::CustomError(
                            format!("key not found in cherry pick: {}", std::any::type_name::<$args>()).into(),
                        ))?,
                    )*
                ))
            }
        }
    }
}

impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
