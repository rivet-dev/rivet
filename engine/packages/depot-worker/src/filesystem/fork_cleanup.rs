//! Local cleanup steps share the existing filesystem mutation and grant guards.
use super::store::{FilesystemStore, LockOperation};
use anyhow::{Result, ensure};
use depot_fs_core::{forks::cleanup::*, keys::slatedb as keys, *};
use rivet_keys::keys::{KeyAddress, codec::FormalKey};

impl FilesystemStore {
	/// `loaded_parent_incarnations` are the parent incarnations the maintainer holds loaded;
	/// see `release_fork_reference`.
	pub async fn fork_cleanup(
		&self,
		work: Work,
		region: u16,
		action: Action,
		loaded_parent_incarnations: &[[u8; 16]],
	) -> Result<Progress> {
		self.admit()?;
		match action {
			Action::Inspect => match work.kind {
				Kind::Check => {
					let _ordered = self
						.lock(
							super::locks::LockRequest::scope(&work.scope),
							LockOperation::Other,
						)
						.await;
					let scope = &work.scope;
					let pending = keys::PendingForkCheckKey::new(
						scope.hash,
						scope.name.clone(),
						scope.incarnation,
					);
					let Some(value) = self.db.get(pending.pack()).await? else {
						return Ok(Progress::Idle);
					};
					let pending = pending.deserialize(value)?;
					let reference = keys::ForkReferenceKey::new(
						scope.hash,
						scope.name.clone(),
						scope.incarnation,
					);
					let value = self
						.db
						.get(reference.pack())
						.await?
						.ok_or_else(|| anyhow::anyhow!("pending fork check lacks reference"))?;
					let FsForkReference::FsForkReferenceHeld(held) =
						reference.deserialize(value)?
					else {
						anyhow::bail!("released reference retains pending check");
					};
					ensure!(
						work.matches(&held.receipt.spec, region)
							&& pending.fork_incarnation == *scope.incarnation.as_bytes()
							&& pending.created_at == held.created_at,
						"fork check identity mismatch"
					);
					self.admit()?;
					Ok(Progress::Check {
						spec: held.receipt.spec,
						created_at: pending.created_at,
					})
				}
				Kind::Release => {
					let scope = &work.scope;
					let address = keys::PendingForkReleaseKey::new(
						scope.hash,
						scope.name.clone(),
						scope.incarnation,
					);
					let Some(value) = self.db.get(address.pack()).await? else {
						return Ok(Progress::Idle);
					};
					let pending = address.deserialize(value)?;
					ensure!(
						work.matches(&pending.spec, region),
						"fork release identity mismatch"
					);
					// This method expires sessions under the mutation lock before
					// deciding whether a captured old chain still needs its hold.
					let spec = self
						.fork_release(
							&pending.spec.destination_volume_key,
							pending.spec.operation_id,
						)
						.await?;
					self.admit()?;
					Ok(spec.map_or(Progress::Idle, Progress::Release))
				}
			},
			Action::Checked { spec, presence } => {
				ensure!(
					work.kind == Kind::Check && work.matches(&spec, region),
					"fork check completion identity mismatch"
				);
				self.complete_fork_check(spec, region, presence, loaded_parent_incarnations)
					.await?
					.map_err(|e| anyhow::anyhow!("fork check rejected: {e:?}"))?;
				Ok(Progress::Idle)
			}
			Action::Released(spec) => {
				ensure!(
					work.kind == Kind::Release && work.matches(&spec, region),
					"fork release completion identity mismatch"
				);
				self.acknowledge_fork_release(&spec).await?;
				Ok(Progress::Idle)
			}
		}
	}
}
