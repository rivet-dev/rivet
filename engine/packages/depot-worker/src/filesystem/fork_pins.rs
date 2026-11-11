//! Fork pin rows of a volume: what a parent in a volume group keeps for the forks of its group.
//!
//! A pin is written by the apply path before the volume's first entry after one or more forks
//! and read back at load to rebuild the record's `fork_pinned_through`. Its `waiting` list only
//! shrinks, in the pin pass: the volume's next pin and its load drop the child groups the
//! maintainer's group cache knows were archived since, and a pin left waiting on no one is
//! deleted in the same batch. A base volume's load reports the children its kept pins wait
//! on, and as the cache learns each one archived the maintainer runs one more pass for that
//! volume. Nothing else deletes a pin, and no event starts a pass, which would need a scan of
//! every volume the group's parent holds. A pass reads only the volume's own pin rows.

use super::{filter::PinnedCut, store::FilesystemStore};
use crate::tablet_maintainer_machine::types::LoadedForkPins;
use anyhow::{Context, Result};
use depot_fs_core::{FsForkPin, keys::slatedb as keys};
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::{IterationOrder, WriteBatch, config::ScanOptions};
use std::collections::BTreeSet;

pub(crate) use crate::tablet_maintainer_machine::types::FORK_PIN_ROW_VERSION;

impl FilesystemStore {
	/// The version a pin row is encoded at: the store's row version, which must already be
	/// one that has pin rows.
	fn new_fork_pin_row_version(&self) -> Result<u16> {
		let version = self.row_version();
		anyhow::ensure!(
			version >= FORK_PIN_ROW_VERSION,
			"fork pins need depot.fs_rows version {FORK_PIN_ROW_VERSION} and the store writes version {version}; the grant's read floor is behind"
		);
		Ok(version)
	}

	/// Every fork pin of `scope`, oldest first.
	pub(crate) async fn fork_pins(
		&self,
		scope: &keys::Scope,
	) -> Result<Vec<(keys::ForkPinKey, FsForkPin)>> {
		self.admit()?;
		let prefix =
			keys::ForkPinRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let mut rows = self.db.scan_prefix(prefix, ..).await?;
		let mut pins = Vec::new();
		while let Some(row) = rows.next().await? {
			let key = keys::ForkPinKey::unpack(&row.key)?;
			let value = key.deserialize(row.value)?;
			pins.push((key, value));
		}
		Ok(pins)
	}

	/// The pin of `scope` covering fork `fork_number`. Pins sort by their cut's sequence, then
	/// their first fork, and a pin taken later stands for later forks, so the newest pin bounds
	/// every pinned fork and the newest pin that starts at or before the fork is the one that can
	/// cover it. The lookup reads from the newest pin back to that one: one row for a fork after
	/// every pin, one or two for a recent fork. A fork inside the pinned range that this pin does
	/// not cover, whose pin a pin pass dropped or whose rows are out of that order, falls back to
	/// reading every pin, so a covering pin is never missed.
	pub(crate) async fn covering_fork_pin(
		&self,
		scope: &keys::Scope,
		fork_number: u64,
	) -> Result<Option<(keys::ForkPinKey, FsForkPin)>> {
		self.admit()?;
		let prefix =
			keys::ForkPinRowsPrefix::new(scope.hash, scope.name.clone(), scope.incarnation, 0)
				.pack();
		let newest_first = ScanOptions::default().with_order(IterationOrder::Descending);
		let mut rows = self
			.db
			.scan_prefix_with_options(prefix, .., &newest_first)
			.await?;
		let mut newest_last_fork = None;
		while let Some(row) = rows.next().await? {
			let key = keys::ForkPinKey::unpack(&row.key)?;
			if newest_last_fork.is_some() && key.first_fork > fork_number {
				continue;
			}
			let pin = key.deserialize(row.value)?;
			let newest = *newest_last_fork.get_or_insert(pin.last_fork);
			if fork_number > newest {
				return Ok(None);
			}
			if key.first_fork > fork_number {
				continue;
			}
			if fork_number <= pin.last_fork {
				return Ok(Some((key, pin)));
			}
			break;
		}
		if newest_last_fork.is_none() {
			return Ok(None);
		}
		crate::metrics::FORK_PIN_LOOKUP_FALLBACKS.inc();
		Ok(self
			.fork_pins(scope)
			.await?
			.into_iter()
			.find(|(key, pin)| key.first_fork <= fork_number && fork_number <= pin.last_fork))
	}

	/// The last fork number `scope` stands pinned for, rebuilt from its rows: the larger of
	/// `created_at_fork`, below which no fork can resolve to this incarnation, and the largest
	/// `last_fork` among its pins. `None` for a volume outside any group.
	pub(crate) async fn fork_pinned_through(
		&self,
		scope: &keys::Scope,
		created_at_fork: Option<u64>,
	) -> Result<Option<u64>> {
		let pins = self.fork_pins(scope).await?;
		Ok(pinned_through(created_at_fork, &pins))
	}

	/// The pin pass at a volume's load: rebuilds its `fork_pinned_through` as
	/// [`Self::fork_pinned_through`] does and drops every group in `archived` from the waiting
	/// lists of the same rows, in one memtable batch. The children the kept pins still wait on
	/// are reported, so the maintainer can learn their status and drop the archived ones in a
	/// later pass.
	/// `stored` is the stored record's `fork_pinned_through`; the result says when a deleted
	/// pin covered forks beyond it and beyond every remaining pin, which a later load could
	/// only rebuild from a record stored again.
	pub(crate) async fn load_fork_pins(
		&self,
		scope: &keys::Scope,
		created_at_fork: Option<u64>,
		stored: Option<u64>,
		archived: &BTreeSet<[u8; 16]>,
	) -> Result<(Option<u64>, LoadedForkPins)> {
		let pins = self.fork_pins(scope).await?;
		let pinned = pinned_through(created_at_fork, &pins);
		let mut batch = WriteBatch::new();
		let (kept, deleted) = if archived.is_empty() {
			(pins, 0)
		} else {
			self.drop_archived(pins, archived, &mut batch)?
		};
		if deleted > 0 {
			self.stage(batch).await?;
			count_dropped(deleted);
			self.refresh_retention_after_pass(scope).await;
		} else if !batch.is_empty() {
			self.stage(batch).await?;
		}
		let kept_through = pinned_through(created_at_fork, &kept).max(stored);
		let waiting_forks: Vec<(u64, u64)> = kept
			.iter()
			.filter(|(_, pin)| !pin.waiting.is_empty())
			.map(|(key, pin)| (key.first_fork, pin.last_fork))
			.collect();
		let waiting: BTreeSet<[u8; 16]> =
			kept.into_iter().flat_map(|(_, pin)| pin.waiting).collect();
		let loaded = LoadedForkPins {
			record_lags: pinned > kept_through,
			waiting: waiting.into_iter().collect(),
			waiting_forks,
		};
		Ok((pinned, loaded))
	}

	/// The pin pass the maintainer starts once its group cache learned that a child a loaded
	/// volume's pins wait on is archived: drops every group in `archived` from the waiting
	/// lists of `scope`'s pins, deleting a pin left waiting on no one, in one memtable batch.
	///
	/// Unlike a pin op it does not wait for the tablet's raised `depot.fs_rows` mark: it
	/// only deletes pin rows or rewrites them at a pin-capable version, and a pin row exists
	/// only because a store already wrote it at such a version. Every reader that could read
	/// the row before can read the rewrite, even on a split child whose fresh record still
	/// names a lower durable rows mark.
	pub(crate) async fn fork_pin_pass(
		&self,
		scope: &keys::Scope,
		archived: &BTreeSet<[u8; 16]>,
	) -> Result<()> {
		self.admit()?;
		let pins = self.fork_pins(scope).await?;
		let mut batch = WriteBatch::new();
		let (_, deleted) = self.drop_archived(pins, archived, &mut batch)?;
		if batch.is_empty() {
			return Ok(());
		}
		self.stage(batch).await?;
		count_dropped(deleted);
		if deleted > 0 {
			self.refresh_retention_after_pass(scope).await;
		}
		Ok(())
	}

	/// Releases the cuts of the pins a pass just deleted from the retention catalog. A reload
	/// that fails leaves them pinned until the volume's next flush reloads the scope, which
	/// keeps versions longer and never shorter, so it is reported and not returned.
	async fn refresh_retention_after_pass(&self, scope: &keys::Scope) {
		if let Err(error) = self.reload_retention_pins(scope).await {
			tracing::warn!(
				?error,
				"the retention catalog kept the cuts of deleted fork pins; the store is closing or loaders kept racing, and the next flush releases them"
			);
		}
	}

	/// Puts the pin row into `batch`. The caller stages the batch and then calls
	/// [`Self::pin_fork_cut`], so the catalog never pins a cut whose row is not in the store.
	pub(crate) fn stage_fork_pin(
		&self,
		scope: &keys::Scope,
		seq: u64,
		first_fork: u64,
		pin: &FsForkPin,
		batch: &mut WriteBatch,
	) -> Result<()> {
		let version = self.new_fork_pin_row_version()?;
		let key = keys::ForkPinKey::new(
			scope.hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			seq,
			first_fork,
		);
		batch.put(
			key.pack(),
			key.serialize(AtVersion::new(pin.clone(), version))?,
		);
		Ok(())
	}

	/// Pins a fork pin's cut in the catalog once its row is staged, and counts a load so a
	/// retention loader whose snapshot predates the row reads again.
	pub(crate) fn pin_fork_cut(&self, scope: &keys::Scope, pin: &FsForkPin) {
		if let Some(retention) = self.catalog.lock().get_mut(scope) {
			retention.pins.insert(PinnedCut::from_cut(&pin.cut));
			retention.loads = retention.loads.wrapping_add(1);
		}
	}

	/// Applies a fork pin, live or in replay: the pin pass drops the `archived` groups from the
	/// scope's earlier pins, then the pin row is written at its cut unless the row is already
	/// there, which a replay of an applied op finds. Both go to the memtable in one batch; the
	/// op is durable in the maintainer log and an ordinary pass makes the rows cold. `archived`
	/// is what the caller knows, not part of the op, so a replay that knows less drops less.
	pub(crate) async fn apply_fork_pin(
		&self,
		scope: &keys::Scope,
		pin: &FsForkPin,
		first_fork: u64,
		archived: &BTreeSet<[u8; 16]>,
	) -> Result<()> {
		self.admit()?;
		let seq = pin
			.cut
			.root_position()
			.context("a fork pin's cut names no root claim log")?
			.volume_seq;
		let mut batch = WriteBatch::new();
		let deleted = if archived.is_empty() {
			0
		} else {
			let pins = self.fork_pins(scope).await?;
			self.drop_archived(pins, archived, &mut batch)?.1
		};
		// A pin waiting on no one would never leave; it is not written. The forks it stands
		// for advance the record's `fork_pinned_through`, which the next pass stores.
		if pin.waiting.is_empty() {
			self.stage(batch).await?;
			count_dropped(deleted);
			if deleted > 0 {
				self.refresh_retention_after_pass(scope).await;
			}
			crate::metrics::GROUP_FORK_PINS
				.with_label_values(&["empty"])
				.inc();
			return Ok(());
		}
		let key = keys::ForkPinKey::new(
			scope.hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			seq,
			first_fork,
		);
		let present = self.db.get(key.pack()).await?.is_some();
		if !present {
			self.stage_fork_pin(scope, seq, first_fork, pin, &mut batch)?;
		}
		self.stage(batch).await?;
		count_dropped(deleted);
		if !present {
			self.pin_fork_cut(scope, pin);
		}
		if deleted > 0 {
			self.refresh_retention_after_pass(scope).await;
		}
		crate::metrics::GROUP_FORK_PINS
			.with_label_values(&[if present { "replayed" } else { "written" }])
			.inc();
		Ok(())
	}

	/// Puts into `batch` the drop of every group in `archived` from the `waiting` of `pins`: a
	/// pin left waiting on no one is deleted, any other changed pin rewritten. Returns the pins
	/// kept, as rewritten, and the number deleted. The caller releases a deleted pin's cut from
	/// the catalog once the batch is staged, and the next compaction drops the versions it kept.
	///
	/// Two passes of one volume never interleave: the load pass, the follow-up pass and the pin
	/// op apply all run from the store queue under a write footprint on the base volume, and a
	/// claim log, which shares the base's rows, runs none of them. Were they to interleave,
	/// each writes a subset of what it read, so the race could only leave a child waiting.
	fn drop_archived(
		&self,
		pins: Vec<(keys::ForkPinKey, FsForkPin)>,
		archived: &BTreeSet<[u8; 16]>,
		batch: &mut WriteBatch,
	) -> Result<(Vec<(keys::ForkPinKey, FsForkPin)>, usize)> {
		let mut kept = Vec::with_capacity(pins.len());
		let mut deleted = 0;
		for (key, mut pin) in pins {
			let before = pin.waiting.len();
			pin.waiting.retain(|group| !archived.contains(group));
			if pin.waiting.len() == before {
				kept.push((key, pin));
				continue;
			}
			if pin.waiting.is_empty() {
				batch.delete(key.pack());
				deleted += 1;
			} else {
				// The row already holds a pin-capable version, so rewriting it at that version
				// shows no reader anything it could not read before, even under a grant whose
				// floor fell below it.
				let version = self.row_version().max(FORK_PIN_ROW_VERSION);
				batch.put(
					key.pack(),
					key.serialize(AtVersion::new(pin.clone(), version))?,
				);
				kept.push((key, pin));
			}
		}
		Ok((kept, deleted))
	}
}

/// The last fork number pinned by `pins` or by the incarnation's creation stamp.
pub(super) fn pinned_through(
	created_at_fork: Option<u64>,
	pins: &[(keys::ForkPinKey, FsForkPin)],
) -> Option<u64> {
	let pinned = pins.iter().map(|(_, pin)| pin.last_fork).max();
	match (created_at_fork, pinned) {
		(None, None) => None,
		(created, pinned) => Some(created.unwrap_or(0).max(pinned.unwrap_or(0))),
	}
}

/// Counts the pins a pin pass deleted once its batch is staged.
fn count_dropped(deleted: usize) {
	if deleted > 0 {
		crate::metrics::GROUP_FORK_PINS
			.with_label_values(&["dropped"])
			.inc_by(deleted as u64);
	}
}
