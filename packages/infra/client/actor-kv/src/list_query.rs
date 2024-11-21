use std::collections::HashMap;

use anyhow::*;
use foundationdb::tuple::Subspace;
use serde::Deserialize;

use crate::{
	entry::EntryBuilder,
	key::{Key, ListKey},
	MAX_KEY_SIZE,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListQuery {
	All,
	RangeInclusive(ListKey, Key),
	RangeExclusive(ListKey, Key),
	Prefix(ListKey),
}

impl ListQuery {
	pub(crate) fn range(&self, subspace: &Subspace) -> (Vec<u8>, Vec<u8>) {
		match self {
			ListQuery::All => subspace.range(),
			ListQuery::RangeInclusive(start, end) => (
				subspace.subspace(&start).range().0,
				subspace.subspace(&end).range().1,
			),
			ListQuery::RangeExclusive(start, end) => (
				subspace.subspace(&start).range().0,
				subspace.subspace(&end).range().1,
			),
			ListQuery::Prefix(prefix) => subspace.subspace(&prefix).range(),
		}
	}

	pub(crate) fn validate(&self) -> Result<()> {
		match self {
			ListQuery::All => {}
			ListQuery::RangeInclusive(start, end) => {
				ensure!(
					start.len() <= MAX_KEY_SIZE,
					"start key is too long (max 2048 bytes)"
				);
				ensure!(
					end.len() <= MAX_KEY_SIZE,
					"end key is too long (max 2048 bytes)"
				);
			}
			ListQuery::RangeExclusive(start, end) => {
				ensure!(
					start.len() <= MAX_KEY_SIZE,
					"startAfter key is too long (max 2048 bytes)"
				);
				ensure!(
					end.len() <= MAX_KEY_SIZE,
					"end key is too long (max 2048 bytes)"
				);
			}
			ListQuery::Prefix(prefix) => {
				ensure!(
					prefix.len() <= MAX_KEY_SIZE,
					"prefix key is too long (max 2048 bytes)"
				);
			}
		}

		Ok(())
	}
}

// Used to short circuit after the
pub struct ListLimitReached(pub HashMap<Key, EntryBuilder>);

impl std::fmt::Debug for ListLimitReached {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ListLimitReached")
	}
}

impl std::fmt::Display for ListLimitReached {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "List limit reached")
	}
}

impl std::error::Error for ListLimitReached {}
