//! Bounded, read-only operator pages over exact volume metadata prefixes.
use super::store::FilesystemStore;
use anyhow::{Result, bail, ensure};
use bytes::Bytes;
use depot_fs_core::keys::slatedb as keys;
use rivet_data::generated::depot_client_protocol_v1::*;
use rivet_keys::keys::{KeyAddress, PrefixAddress, codec::FormalKey};
use std::ops::Bound;
use uuid::Uuid;

impl FilesystemStore {
	pub async fn repair_diagnostics(
		&self,
		request: RepairDiagnosticsRequest,
	) -> Result<RepairDiagnosticsPage> {
		self.admit()?;
		ensure!(
			request.kind != RepairDiagnosticsKind::Activity,
			"activity is read from maintainer state, not the store"
		);
		let scope = keys::Scope::new(
			&request.volume_log_key.volume_key,
			Uuid::from_bytes(request.volume_log_key.volume_incarnation),
		);
		let limit = request.limit.clamp(1, 100) as usize;
		if request.kind == RepairDiagnosticsKind::Incarnations {
			return self
				.repair_incarnations(&scope, request.after.as_deref(), limit)
				.await;
		}
		if request.kind == RepairDiagnosticsKind::Pins {
			let now = self.clock.observe()?.boot_ms;
			let after = request.after.as_deref();
			ensure!(after.is_none_or(|v| v.len() == 16), "invalid pin cursor");
			let sessions = self.sessions.lock();
			let mut rows = Vec::new();
			for session in sessions.values().filter(|s| {
				s.scope == scope
					&& s.deadline_boot_ms() > now
					&& after.is_none_or(|a| s.id.as_bytes().as_slice() > a)
			}) {
				rows.push(RepairDiagnosticRow {
					key: session.id.as_bytes().to_vec().into(),
					raw: Vec::new().into(),
					decoded: Some(RepairDiagnosticValue::RepairPin(RepairPin {
						session_id: *session.id.as_bytes(),
						as_of: session.as_of,
						applied: session.applied.clone(),
						expires_boot_ms: session.deadline_boot_ms(),
						base_chain: session.base_chain.clone(),
					})),
					decode_error: None,
				});
				if rows.len() == limit {
					break;
				}
			}
			self.admit()?;
			return Ok(page(rows, limit));
		}
		if request.kind == RepairDiagnosticsKind::Alarms {
			ensure!(
				request.after.is_none(),
				"alarms are an exact lookup without a cursor"
			);
			let address = depot_types::keys::slatedb::AlarmCurrentKey::new(
				scope.hash,
				scope.name,
				scope.incarnation,
			);
			let key = address.pack();
			let row = self.db.get(&key).await?.map(|raw| {
				let decoded = address
					.deserialize(raw.clone())
					.map(RepairDiagnosticValue::AlarmCurrent);
				diagnostic_row(key, raw, decoded)
			});
			self.admit()?;
			return Ok(RepairDiagnosticsPage {
				rows: row.into_iter().collect(),
				after: None,
			});
		}
		let prefix = match request.kind {
			RepairDiagnosticsKind::Forks => {
				keys::ForkHoldRowsPrefix::new(scope.hash, scope.name, scope.incarnation, 0).pack()
			}
			RepairDiagnosticsKind::Pending => {
				keys::PendingVolumePrefix::new(scope.hash, scope.name).pack()
			}
			RepairDiagnosticsKind::Pins
			| RepairDiagnosticsKind::Alarms
			| RepairDiagnosticsKind::Incarnations
			| RepairDiagnosticsKind::Activity => bail!("not a metadata prefix page"),
		};
		let after = request.after.as_deref();
		ensure!(
			after.is_none_or(|v| v.starts_with(&prefix)),
			"foreign diagnostic cursor"
		);
		let lower = after.map_or(Bound::Unbounded, |v| {
			Bound::Excluded(Bytes::copy_from_slice(&v[prefix.len()..]))
		});
		let snapshot = self.db.snapshot().await?;
		let mut scan = snapshot
			.scan_prefix(prefix, (lower, Bound::<Bytes>::Unbounded))
			.await?;
		let mut rows = Vec::new();
		while rows.len() < limit {
			let Some(row) = scan.next().await? else {
				self.admit()?;
				return Ok(RepairDiagnosticsPage { rows, after: None });
			};
			let decoded = match request.kind {
				RepairDiagnosticsKind::Forks => keys::ForkHoldKey::unpack(&row.key)
					.and_then(|k| k.deserialize(row.value.clone()))
					.map(RepairDiagnosticValue::FsForkHold),
				RepairDiagnosticsKind::Pending => decode_pending(&row.key, row.value.clone()),
				RepairDiagnosticsKind::Pins
				| RepairDiagnosticsKind::Alarms
				| RepairDiagnosticsKind::Incarnations
				| RepairDiagnosticsKind::Activity => bail!("not a metadata prefix page"),
			};
			rows.push(diagnostic_row(row.key, row.value, decoded));
		}
		self.admit()?;
		Ok(page(rows, limit))
	}

	/// Seek only to the two record keys of each incarnation, skipping its file rows.
	async fn repair_incarnations(
		&self,
		scope: &keys::Scope,
		after: Option<&[u8]>,
		limit: usize,
	) -> Result<RepairDiagnosticsPage> {
		let prefix = keys::VolumePrefix::new(scope.hash, scope.name.clone()).pack();
		ensure!(
			after.is_none_or(|v| v.starts_with(&prefix)),
			"foreign incarnation cursor"
		);
		let first =
			keys::IncarnationPrefix::new(scope.hash, scope.name.clone(), Uuid::nil()).pack();
		let lower = after.map_or_else(
			|| Bound::Included(Bytes::copy_from_slice(&first[prefix.len()..])),
			|v| Bound::Excluded(Bytes::copy_from_slice(&v[prefix.len()..])),
		);
		let snapshot = self.db.snapshot().await?;
		let mut scan = snapshot
			.scan_prefix(prefix, (lower, Bound::<Bytes>::Unbounded))
			.await?;
		let mut rows = Vec::new();
		let mut cursor = None;
		for _ in 0..limit.saturating_mul(4) {
			let Some(row) = scan.next().await? else {
				self.admit()?;
				return Ok(RepairDiagnosticsPage { rows, after: None });
			};
			cursor = Some(row.key.clone().into());
			let Some(found) = keys::decode_scope(&row.key) else {
				break;
			};
			let own = keys::VolumeIncarnationRecordKey::new(
				found.hash,
				found.name.clone(),
				found.incarnation,
			);
			let archive =
				keys::IncarnationRecordKey::new(found.hash, found.name.clone(), found.incarnation);
			let own_key = own.pack();
			let archive_key = archive.pack();
			if row.key == own_key {
				let value = own
					.deserialize(row.value.clone())
					.map(RepairDiagnosticValue::FsVolumeRecord);
				rows.push(diagnostic_row(row.key.clone(), row.value, value));
			} else if row.key == archive_key {
				let value = archive
					.deserialize(row.value.clone())
					.map(RepairDiagnosticValue::FsIncarnationRecord);
				rows.push(diagnostic_row(row.key.clone(), row.value, value));
			}
			if rows.len() == limit {
				break;
			}
			let target = [own_key, archive_key]
				.into_iter()
				.filter(|k| *k > row.key)
				.min()
				.unwrap_or_else(|| {
					let mut end =
						keys::IncarnationPrefix::new(found.hash, found.name, found.incarnation)
							.pack()
							.to_vec();
					end.push(0xff);
					Bytes::from(end)
				});
			scan.seek(&target).await?;
		}
		self.admit()?;
		Ok(RepairDiagnosticsPage {
			rows,
			after: cursor,
		})
	}
}

fn page(rows: Vec<RepairDiagnosticRow>, limit: usize) -> RepairDiagnosticsPage {
	// A full page continues after its last row; a short one is the last page.
	let after = if rows.len() == limit {
		rows.last().map(|row| row.key.clone())
	} else {
		None
	};
	RepairDiagnosticsPage { rows, after }
}

fn diagnostic_row(
	key: Bytes,
	raw: Bytes,
	decoded: Result<RepairDiagnosticValue>,
) -> RepairDiagnosticRow {
	let (decoded, decode_error) = match decoded {
		Ok(value) => (Some(value), None),
		Err(error) => (None, Some(error.to_string().into())),
	};
	RepairDiagnosticRow {
		key: key.into(),
		raw: raw.into(),
		decoded,
		decode_error,
	}
}

fn decode_pending(key: &Bytes, raw: Bytes) -> Result<RepairDiagnosticValue> {
	macro_rules! decode {
		($key:ident, $variant:ident) => {
			if let Ok(address) = keys::$key::unpack(key) {
				return address
					.deserialize(raw)
					.map(RepairDiagnosticValue::$variant);
			}
		};
	}
	decode!(PendingForkKey, FsPendingFork);
	decode!(PendingForkCheckKey, FsPendingForkCheck);
	decode!(PendingForkReleaseKey, FsPendingForkRelease);
	decode!(PendingMaterializeKey, FsPendingMaterialize);
	decode!(PendingSweepKey, FsPendingSweep);
	if let Ok(address) = keys::PendingDeleteKey::unpack(key) {
		address.deserialize(raw)?;
		return Ok(RepairDiagnosticValue::RepairPendingDelete(
			RepairPendingDelete {
				incarnation: *address.incarnation.as_bytes(),
			},
		));
	}
	bail!("unknown pending metadata key")
}
