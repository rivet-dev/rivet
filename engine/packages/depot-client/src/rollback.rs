//! Rolls an owned filesystem back to one of its named checkpoints, reading the checkpoint
//! through a read session of the volume's current incarnation. See
//! [`crate::filesystem::rollback`] for how the writes are planned and committed.

use crate::{
	Client,
	filesystem::{self, CheckpointImage, Filesystem, RollbackOutcome},
};
use depot_fs_core::store::{Row, RowKey};
use depot_types::VolumeKey;
use futures_util::future::BoxFuture;
use rivet_data::generated::{
	depot_client_protocol_v2::*,
	shared_depot_fs_read_v2::*,
	shared_depot_fs_v2::FsCut,
	shared_depot_types_v1::{VolumeLogKey, VolumeLogPosition},
};
use std::sync::Arc;
use std::time::Duration;

/// How long one read of the checkpoint may take, including its retries.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Attempts of one checkpoint read before its error is returned. A read changes nothing, so
/// it is sent again after an expired session, a moved maintainer or shed load.
const READ_ATTEMPTS: u32 = 6;

/// The first pause between read attempts; each later pause doubles it.
const READ_RETRY_BASE_MS: u64 = 50;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// The volume has no named checkpoint by that name in its current incarnation.
	#[error("checkpoint not found")]
	CheckpointNotFound,
	#[error(transparent)]
	Inspect(anyhow::Error),
	#[error(transparent)]
	Filesystem(#[from] filesystem::Error),
}

impl Client {
	/// Looks up checkpoint `name` of the volume's current incarnation. Its `checkpoint` is
	/// absent when no checkpoint, named or held by a fork, uses the name.
	pub async fn checkpoint_inspect(
		&self,
		key: VolumeKey,
		name: Vec<u8>,
	) -> anyhow::Result<CheckpointInspection> {
		let transport = self.volume_handle()?.read_requests();
		let deadline = transport.deadline()?;
		match crate::checkpoints::request(
			&transport,
			self.env().clock().as_ref(),
			key,
			CheckpointOperation::CheckpointInspect(CheckpointInspect { name: name.into() }),
			deadline,
		)
		.await?
		{
			VolumeCheckpointResponse::CheckpointInspection(inspection) => Ok(inspection),
			VolumeCheckpointResponse::FsCheckpointInfo(_)
			| VolumeCheckpointResponse::CheckpointPage(_)
			| VolumeCheckpointResponse::CheckpointUpdated
			| VolumeCheckpointResponse::CheckpointError(_) => Err(crate::checkpoints::Error::Unknown.into()),
		}
	}

	/// Rolls `fs`, which the caller owns and keeps every other writer away from, back to its
	/// named checkpoint `name`, and returns what the rollback committed.
	pub async fn rollback_filesystem(
		&self,
		fs: &Arc<Filesystem>,
		name: Vec<u8>,
	) -> Result<RollbackOutcome, Error> {
		let key = fs.volume_log_key().clone();
		let inspection = self
			.checkpoint_inspect(key.0.volume_key.clone(), name)
			.await
			.map_err(Error::Inspect)?;
		let Some(info) = inspection.checkpoint.filter(|info| {
			info.checkpoint.named && info.volume_incarnation == key.0.volume_incarnation
		}) else {
			return Err(Error::CheckpointNotFound);
		};
		// A volume with one claim checkpoints a cut over claim log 0 alone.
		let position = info
			.position
			.into_single()
			.map_err(|_| filesystem::Error::Corrupt)?;
		let image = SessionImage::open(self.clone(), key.0.clone(), position).await?;
		let result = fs.rollback(&image, self.rollback_budget()).await;
		image.close().await;
		Ok(result?)
	}
}

/// A checkpoint of one incarnation, read through a read session pinned at its position. An
/// expired session is opened again at the same position, which the checkpoint keeps readable.
pub struct SessionImage {
	client: Client,
	key: VolumeLogKey,
	position: VolumeLogPosition,
	session: tokio::sync::Mutex<[u8; 16]>,
}

impl SessionImage {
	pub async fn open(
		client: Client,
		key: VolumeLogKey,
		position: VolumeLogPosition,
	) -> filesystem::Result<Self> {
		let image = Self {
			client,
			key,
			position,
			session: tokio::sync::Mutex::new([0; 16]),
		};
		let session = image.open_session().await?;
		*image.session.lock().await = session;
		Ok(image)
	}

	/// Releases the session. A close that fails leaves it to expire on the maintainer.
	pub async fn close(&self) {
		let session = *self.session.lock().await;
		let _ = self
			.send(FsReadOperation::FsReadClose(FsReadClose { session }))
			.await;
	}

	async fn open_session(&self) -> filesystem::Result<[u8; 16]> {
		let response = self
			.send(FsReadOperation::FsReadOpen(FsReadOpen {
				cut: Some(FsCut::single(self.position.clone())),
				prefetch: None,
			}))
			.await?;
		let FsReadResponse::FsReadOpened(opened) = response else {
			return Err(filesystem::Error::Corrupt);
		};
		if opened.volume_log_key != self.key || opened.as_of != self.position.volume_seq {
			return Err(filesystem::Error::Corrupt);
		}
		Ok(opened.session.ok_or(filesystem::Error::Corrupt)?.id)
	}

	/// Sends one read and maps a refusal to its filesystem error.
	async fn send(&self, operation: FsReadOperation) -> filesystem::Result<FsReadResponse> {
		let clock = self.client.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), READ_TIMEOUT)
			.map_err(|_| filesystem::Error::Unavailable)?;
		let response = self
			.client
			.read_filesystem(
				FsReadRequest {
					target: FsReadTarget::VolumeLogKey(self.key.clone()),
					client_epoch: None,
					operation,
				},
				deadline,
			)
			.await
			.map_err(|_| filesystem::Error::ColdReadRefused)?;
		match response {
			FsReadResponse::FsReadError(error) => Err(match error {
				FsReadError::TooOld => filesystem::Error::TooOld,
				FsReadError::Lagging => filesystem::Error::Lagging,
				FsReadError::Fenced => filesystem::Error::Fenced,
				FsReadError::TooLarge => filesystem::Error::TooLarge,
				FsReadError::Overloaded => filesystem::Error::Overloaded,
				FsReadError::NotFound => depot_fs_core::Error::NotFound.into(),
				FsReadError::Corrupt => filesystem::Error::Corrupt,
				FsReadError::Unavailable => filesystem::Error::Unavailable,
				FsReadError::BaseUnavailable => filesystem::Error::BaseUnavailable,
				FsReadError::NotServing => filesystem::Error::ColdReadRefused,
			}),
			response => Ok(response),
		}
	}

	/// Reads `operation(session)` through the session, opening it again when it expired and
	/// sending the read again after a refusal that ran nothing.
	async fn rows(
		&self,
		operation: impl Fn(FsReadAt) -> FsReadOperation,
	) -> filesystem::Result<Vec<(RowKey, Row)>> {
		let clock = self.client.env().clock();
		let mut attempt = 0;
		loop {
			let session = *self.session.lock().await;
			let result = self
				.send(operation(FsReadAt::FsSessionRead(FsSessionRead {
					session,
				})))
				.await;
			let error = match result {
				Ok(FsReadResponse::FsReadRows(rows)) => {
					if rows.session != Some(session) || rows.as_of != self.position.volume_seq {
						return Err(filesystem::Error::Corrupt);
					}
					return rows
						.rows
						.into_iter()
						.map(|row| depot_fs_core::wire::decode_row(row).map_err(Into::into))
						.collect();
				}
				Ok(_) => return Err(filesystem::Error::Corrupt),
				Err(error) => error,
			};
			attempt += 1;
			let retryable = matches!(
				error,
				filesystem::Error::TooOld
					| filesystem::Error::Lagging
					| filesystem::Error::Overloaded
					| filesystem::Error::ColdReadRefused
					| filesystem::Error::Unavailable
			);
			if !retryable || attempt >= READ_ATTEMPTS {
				return Err(error);
			}
			let pause = READ_RETRY_BASE_MS << (attempt - 1);
			rivet_env::sleep(clock.as_ref(), Duration::from_millis(pause))
				.await
				.map_err(|_| filesystem::Error::Unavailable)?;
			if matches!(error, filesystem::Error::TooOld) {
				let mut current = self.session.lock().await;
				if *current == session {
					*current = self.open_session().await?;
				}
			}
		}
	}
}

impl CheckpointImage for SessionImage {
	fn seq(&self) -> u64 {
		self.position.volume_seq
	}

	fn scan_metadata(
		&self,
		after: Option<RowKey>,
		limit: usize,
	) -> BoxFuture<'_, filesystem::Result<Vec<(RowKey, Row)>>> {
		Box::pin(async move {
			let limit = u32::try_from(limit).map_err(|_| filesystem::Error::TooLarge)?;
			let after = after.map(FsRowKey::from);
			self.rows(|at| {
				FsReadOperation::FsReadScan(FsReadScan {
					at,
					range: FsRowRange::FsAllMetadataRows,
					after: after.clone(),
					limit,
				})
			})
			.await
		})
	}

	fn read_chunks(
		&self,
		ino: u64,
		first: u64,
		count: u64,
	) -> BoxFuture<'_, filesystem::Result<Vec<Option<bytes::Bytes>>>> {
		Box::pin(async move {
			let count = u32::try_from(count).map_err(|_| filesystem::Error::TooLarge)?;
			let rows = self
				.rows(|at| {
					FsReadOperation::FsReadChunks(FsReadChunks {
						at,
						ino,
						ranges: vec![FsChunkRange { first, count }],
					})
				})
				.await?;
			let mut chunks = vec![None; count as usize];
			for (key, row) in rows {
				let (RowKey::Chunk(found, index), Row::Bytes(bytes)) = (key, row) else {
					return Err(filesystem::Error::Corrupt);
				};
				if found != ino || index < first || index - first >= u64::from(count) {
					return Err(filesystem::Error::Corrupt);
				}
				chunks[(index - first) as usize] = Some(bytes);
			}
			Ok(chunks)
		})
	}
}
