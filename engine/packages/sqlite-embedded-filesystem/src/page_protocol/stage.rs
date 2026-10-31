//! A stage owns an epoch-bound FS transaction and compact retry receipts only.
use super::{
	check_write_head,
	database::{Database, PAGE},
};
use anyhow::{Context, Result, ensure};
use depot_client::filesystem::{FileEdit, FileWrite, TxnHandle};
use rivet_envoy_protocol as p;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub struct Stage {
	txid: u64,
	transaction: TxnHandle,
	segments: BTreeMap<u32, Segment>,
	bytes: usize,
	prefetch: sqlite_prefetch::CommitBuilder,
}
struct Segment {
	digest: [u8; 32],
	last_pgno: u32,
}
pub(super) async fn execute(
	stage: &mut Option<Stage>,
	pages: &mut Database,
	message: p::ToRivet,
) -> Result<p::ToEnvoy> {
	match message {
		p::ToRivet::ToRivetSqliteCommitStageBeginRequest(req) => {
			check_write_head(req.data.expected_head_txid, pages)?;
			let base = pages.root.txid;
			let txid = base.checked_add(1).context("SQLite head exhausted")?;
			if !stage.as_ref().is_some_and(|s| s.txid == txid) {
				abort(stage).await?;
				*stage = Some(Stage {
					txid,
					transaction: pages
						.fs
						.begin_staged_write(pages.ino, base, [pages.prefetch_ino])
						.await?,
					segments: BTreeMap::new(),
					bytes: 0,
					prefetch: pages.prefetcher.commit_builder(),
				});
			}
			Ok(p::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				p::ToEnvoySqliteCommitStageBeginResponse {
					request_id: req.request_id,
					data: p::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
						p::SqliteCommitStageBeginOk { txid },
					),
				},
			))
		}
		p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(req) => {
			let stage = stage
				.as_mut()
				.context("SQLite stage missing; restart the transaction")?;
			ensure!(
				stage.txid == req.data.txid && pages.root.txid.checked_add(1) == Some(stage.txid),
				"SQLite stage fenced"
			);
			ensure!(
				req.data.first_pgno % sqlite_types::SHARD_SIZE == 0
					&& !req.data.dirty_pages.is_empty(),
				"invalid SQLite segment"
			);
			let end = u64::from(req.data.first_pgno)
				+ u64::from(sqlite_types::SHARD_SIZE)
					* u64::from(sqlite_types::COMMIT_SEGMENT_MAX_SHARDS);
			let mut last = 0;
			let mut digest = Sha256::new();
			for page in &req.data.dirty_pages {
				ensure!(
					page.pgno > last
						&& page.pgno >= req.data.first_pgno
						&& u64::from(page.pgno) < end
						&& page.bytes.len() == PAGE,
					"invalid SQLite segment page"
				);
				last = page.pgno;
				digest.update(page.pgno.to_be_bytes());
				digest.update(&page.bytes);
			}
			let digest: [u8; 32] = digest.finalize().into();
			let bytes = req.data.dirty_pages.len() * PAGE;
			if let Some(old) = stage.segments.get(&req.data.first_pgno) {
				ensure!(old.digest == digest, "SQLite retry changed segment");
			} else {
				ensure!(
					stage.bytes + bytes
						<= pages
							.fs
							.max_transaction_bytes()
							.min(sqlite_types::MAX_COMMIT_DIRTY_PAGES * PAGE),
					"SQLite transaction byte limit exceeded"
				);
				ensure!(
					stage
						.segments
						.iter()
						.all(|(&start, old)| last < start || req.data.first_pgno > old.last_pgno),
					"overlapping SQLite segments"
				);
				for page in &req.data.dirty_pages {
					pages
						.prefetcher
						.add_dirty(&mut stage.prefetch, page.pgno, &page.bytes);
				}
				let edits = req
					.data
					.dirty_pages
					.into_iter()
					.map(|p| {
						FileEdit::Write(FileWrite {
							offset: u64::from(p.pgno - 1) * PAGE as u64,
							bytes: p.bytes,
						})
					})
					.collect::<Vec<_>>();
				// Admission is sufficient: the final commit ticket covers every segment.
				stage.transaction.edit_file(pages.ino, &edits).await?;
				stage.bytes += bytes;
				stage.segments.insert(
					req.data.first_pgno,
					Segment {
						digest,
						last_pgno: last,
					},
				);
			}
			Ok(p::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(
				p::ToEnvoySqliteCommitStageSegmentResponse {
					request_id: req.request_id,
					data: p::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(
						p::SqliteCommitStageSegmentOk {
							staged_bytes: bytes as u64,
						},
					),
				},
			))
		}
		p::ToRivet::ToRivetSqliteCommitFinalizeRequest(req) => {
			if let Some(current) = stage.as_ref() {
				ensure!(
					current.txid == req.data.txid
						&& pages.root.txid.checked_add(1) == Some(current.txid)
						&& current.segments.keys().copied().collect::<Vec<_>>()
							== req.data.segment_first_pgnos,
					"SQLite finalize does not match stage"
				);
				let plan = pages
					.prefetch_commit(&current.prefetch, req.data.new_db_size_pages, 0)
					.await?;
				let ops = pages.prefetch_ops(&plan);
				let Some(current) = stage.as_mut() else {
					anyhow::bail!("SQLite stage missing; restart the transaction");
				};
				let committed = current
					.transaction
					.commit_file_edits_with_ops(
						pages.ino,
						&pages.final_edits(req.data.new_db_size_pages),
						&[],
						ops,
					)
					.await;
				// A commit refused before it was admitted keeps the stage, so the identical
				// finalize commits it. After any other outcome the stage is done; dropping a
				// transaction that did not commit aborts it once its outcome is known.
				if !committed
					.as_ref()
					.is_err_and(|error| *error == depot_client::filesystem::Error::ColdReadRefused)
				{
					stage.take();
				}
				let seq = match committed {
					Ok(seq) => seq,
					Err(error) => {
						pages.prefetcher.commit_failed();
						return Err(error.into());
					}
				};
				pages.prefetch_committed(plan);
				pages.committed(seq, req.data.new_db_size_pages);
			} else {
				// The head this request read decides whether the lost stage committed. The
				// serialized actor lane is the only writer, and a new owner has already
				// replayed its claim entry, so no unresolved old commit can race that read.
				let base = req
					.data
					.txid
					.checked_sub(1)
					.context("invalid SQLite stage id")?;
				if pages.root.txid <= base {
					pages.fs.abort_staged_write(pages.ino, base).await?;
					anyhow::bail!("SQLite stage did not commit; restart the transaction");
				}
				ensure!(
					pages.root.size_pages == req.data.new_db_size_pages,
					"SQLite finalize retry changed file size"
				);
			}
			Ok(p::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				p::ToEnvoySqliteCommitFinalizeResponse {
					request_id: req.request_id,
					data: p::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
						p::SqliteCommitFinalizeOk {
							head_txid: Some(pages.root.txid),
						},
					),
				},
			))
		}
		_ => anyhow::bail!("unexpected SQLite request"),
	}
}

pub(super) async fn abort(stage: &mut Option<Stage>) -> Result<()> {
	if let Some(stage) = stage.take() {
		stage.transaction.abort().await?;
	}
	Ok(())
}
