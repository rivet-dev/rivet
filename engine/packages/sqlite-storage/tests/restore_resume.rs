mod support;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{RestoreMode, RestoreTarget},
	compactor::{CheckpointOutcome, handle_restore, restore::test_hooks},
	keys::{checkpoint_meta_key, meta_restore_in_progress_key, pidx_delta_key, shard_key},
	types::{decode_checkpoint_meta, decode_restore_marker},
};
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

static RESTORE_HOOK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn restore_resume_after_pod_failure() -> Result<()> {
	let _lock = RESTORE_HOOK_LOCK.lock().await;
	let db = support::test_db("sqlite-restore-resume-").await?;
	let actor_id = "restore-resume";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(2, 0x22)], 4, 200).await?;
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;
	let op_id = Uuid::new_v4();
	support::create_restore_record(db.clone(), actor_id, op_id).await?;
	let holder = NodeId::new();
	let (_guard, reached, release) = test_hooks::pause_after_marker_clear(actor_id);
	let restore_task = tokio::spawn(handle_restore(
		db.clone(),
		op_id,
		actor_id.to_string(),
		RestoreTarget::Txid(2),
		RestoreMode::Apply,
		holder,
		CancellationToken::new(),
	));

	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	assert!(support::marker_exists(db.clone(), actor_id).await?);
	restore_task.abort();
	release.notify_waiters();
	let _ = restore_task.await;

	handle_restore(
		db.clone(),
		op_id,
		actor_id.to_string(),
		RestoreTarget::Txid(2),
		RestoreMode::Apply,
		holder,
		CancellationToken::new(),
	)
	.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; 4096]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; 4096]));
	assert!(!support::marker_exists(db, actor_id).await?);
	Ok(())
}

#[tokio::test]
async fn restore_resume_pins_checkpoint_refcount() -> Result<()> {
	let _lock = RESTORE_HOOK_LOCK.lock().await;
	let db = support::test_db("sqlite-restore-resume-pin-").await?;
	let actor_id = "restore-resume-pin";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	let op_id = Uuid::new_v4();
	support::create_restore_record(db.clone(), actor_id, op_id).await?;
	let holder = NodeId::new();
	let (_guard, reached, release) = test_hooks::pause_after_marker_clear(actor_id);
	let restore_task = tokio::spawn(handle_restore(
		db.clone(),
		op_id,
		actor_id.to_string(),
		RestoreTarget::Txid(1),
		RestoreMode::Apply,
		holder,
		CancellationToken::new(),
	));
	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	restore_task.abort();
	release.notify_waiters();
	let _ = restore_task.await;

	let marker = db
		.run({
			let actor_id = actor_id.to_string();
			move |tx| {
				let actor_id = actor_id.clone();
				async move {
					let bytes = tx
						.informal()
						.get(&meta_restore_in_progress_key(&actor_id), Snapshot)
						.await?
						.expect("restore marker should exist");
					decode_restore_marker(&bytes)
				}
			}
		})
		.await?;
	assert_eq!(marker.ckp_txid, 1);

	handle_restore(
		db.clone(),
		op_id,
		actor_id.to_string(),
		RestoreTarget::Txid(1),
		RestoreMode::Apply,
		holder,
		CancellationToken::new(),
	)
	.await?;
	let meta = db
		.run({
			let actor_id = actor_id.to_string();
			move |tx| {
				let actor_id = actor_id.clone();
				async move {
					let bytes = tx
						.informal()
						.get(&checkpoint_meta_key(&actor_id, 1), Snapshot)
						.await?
						.expect("checkpoint meta should exist");
					decode_checkpoint_meta(&bytes)
				}
			}
		})
		.await?;
	assert_eq!(meta.refcount, 0);
	Ok(())
}

#[tokio::test]
async fn restore_marker_in_same_tx_as_clear() -> Result<()> {
	let _lock = RESTORE_HOOK_LOCK.lock().await;
	let db = support::test_db("sqlite-restore-marker-clear-").await?;
	let actor_id = "restore-marker-clear";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	db.run({
		let actor_id = actor_id.to_string();
		move |tx| {
			let actor_id = actor_id.clone();
			async move {
				tx.informal().set(&shard_key(&actor_id, 0), b"stale-shard");
				Ok(())
			}
		}
	})
	.await?;
	let op_id = Uuid::new_v4();
	support::create_restore_record(db.clone(), actor_id, op_id).await?;
	let holder = NodeId::new();
	let (_guard, reached, release) = test_hooks::pause_after_marker_clear(actor_id);
	let restore_task = tokio::spawn(handle_restore(
		db.clone(),
		op_id,
		actor_id.to_string(),
		RestoreTarget::Txid(1),
		RestoreMode::Apply,
		holder,
		CancellationToken::new(),
	));

	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	let (marker_exists, pidx_exists, shard_exists) = db
		.run({
			let actor_id = actor_id.to_string();
			move |tx| {
				let actor_id = actor_id.clone();
				async move {
					let marker = tx
						.informal()
						.get(&meta_restore_in_progress_key(&actor_id), Snapshot)
						.await?
						.is_some();
					let pidx = tx
						.informal()
						.get(&pidx_delta_key(&actor_id, 1), Snapshot)
						.await?
						.is_some();
					let shard = tx
						.informal()
						.get(&shard_key(&actor_id, 0), Snapshot)
						.await?
						.is_some();
					Ok((marker, pidx, shard))
				}
			}
		})
		.await?;
	assert!(marker_exists);
	assert!(!pidx_exists);
	assert!(!shard_exists);

	release.notify_waiters();
	restore_task.await??;
	Ok(())
}
