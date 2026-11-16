//! Writerless command reads served by maintainer point reads.
//!
//! Each `get` is one point read: the maintainer opens a snapshot, reads the key and releases
//! the snapshot within the request, so no session is opened or closed. The first read takes the
//! latest applied snapshot, which can lag the committed log by the apply delay; later reads of
//! the same command pin that snapshot's position, so one command sees one snapshot. Only
//! commands whose contract tolerates that staleness are registered as read commands.
use anyhow::Result;
use depot_client::filesystem::remote::ReadTransport;
use rivet_data::generated::{
	shared_depot_fs_read_v2::{
		FsReadError, FsReadKeys, FsReadOperation, FsReadRequest, FsReadResponse, FsReadTarget,
		FsReadValues,
	},
	shared_depot_fs_v2::FsCut,
	shared_depot_types_v1::{VolumeLogKey, VolumeLogPosition},
};
use std::sync::Arc;

/// Why a writerless read could not be served. The host maps it to the command's response.
#[derive(Clone, Copy, Debug)]
pub(crate) enum WriterlessReadError {
	/// The volume has no stored row and no loaded incarnation.
	NotFound,
	/// The maintainer refused the read: it is out of read capacity, or a lifecycle request for
	/// the volume is pending. The host returns a retryable refusal and never opens the writer
	/// for a read.
	Overloaded,
	/// The value cannot be returned in one response.
	TooLarge,
	/// Any other failure, including an answer that breaks the point read contract. The caller
	/// retries the command.
	Unavailable,
}

impl std::fmt::Display for WriterlessReadError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			WriterlessReadError::NotFound => write!(f, "writerless read found no volume"),
			WriterlessReadError::Overloaded => write!(f, "writerless read refused as overloaded"),
			WriterlessReadError::TooLarge => write!(f, "writerless read value too large"),
			WriterlessReadError::Unavailable => write!(f, "writerless read unavailable"),
		}
	}
}

impl std::error::Error for WriterlessReadError {}

/// The snapshot a command's first point read observed.
#[derive(Clone)]
struct Pinned {
	volume_log_key: VolumeLogKey,
	position: VolumeLogPosition,
}

/// Point reads of one command volume's `kv` file, all from the first read's snapshot.
pub(crate) struct WriterlessRead {
	transport: Arc<dyn ReadTransport>,
	volume_key: depot_types::VolumeKey,
	kv_name: Vec<u8>,
	deadline_boot_ms: u64,
	/// Held across the first read so concurrent first reads of one command share a snapshot.
	pinned: tokio::sync::Mutex<Option<Pinned>>,
}

impl WriterlessRead {
	pub(crate) fn new(
		transport: Arc<dyn ReadTransport>,
		volume_key: depot_types::VolumeKey,
		kv_name: &[u8],
		deadline_boot_ms: u64,
	) -> Self {
		Self {
			transport,
			volume_key,
			kv_name: kv_name.to_vec(),
			deadline_boot_ms,
			pinned: tokio::sync::Mutex::new(None),
		}
	}

	/// Reads one key of the `kv` file in one round trip. A volume without a `kv` file holds
	/// no keys.
	pub(crate) async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		let mut values = self.get_many(std::slice::from_ref(&key.to_vec())).await?;
		Ok(values.pop().flatten())
	}

	/// Reads `keys` of the `kv` file in one round trip, in their order; see [`Self::get`].
	pub(crate) async fn get_many(&self, keys: &[Vec<u8>]) -> Result<Vec<Option<Vec<u8>>>> {
		Ok(self.get_many_with_lag(keys).await?.0)
	}

	/// [`Self::get_many`], with how many volume seqs the snapshot lagged the highest commit the
	/// maintainer knew of when it answered.
	pub(crate) async fn get_many_with_lag(
		&self,
		keys: &[Vec<u8>],
	) -> Result<(Vec<Option<Vec<u8>>>, u64)> {
		let mut pinned = self.pinned.lock().await;
		let current = pinned.clone();
		if current.is_some() {
			drop(pinned);
			let (_, values, lag) = self.point_read(current, keys).await?;
			return Ok((values, lag));
		}
		let (observed, values, lag) = self.point_read(None, keys).await?;
		*pinned = Some(observed);
		Ok((values, lag))
	}

	async fn point_read(
		&self,
		pinned: Option<Pinned>,
		keys: &[Vec<u8>],
	) -> Result<(Pinned, Vec<Option<Vec<u8>>>, u64)> {
		let (target, position) = match &pinned {
			Some(pinned) => (
				FsReadTarget::VolumeLogKey(pinned.volume_log_key.clone()),
				Some(pinned.position.clone()),
			),
			None => (FsReadTarget::VolumeKey(self.volume_key.clone()), None),
		};
		let request = FsReadRequest {
			target,
			client_epoch: None,
			operation: FsReadOperation::FsReadKeys(FsReadKeys {
				cut: position.map(FsCut::single),
				file: self.kv_name.clone().into(),
				keys: keys.iter().map(|key| key.clone().into()).collect(),
			}),
		};
		let response = self.transport.read_at(request, self.deadline_boot_ms).await;
		let values = match response {
			Ok(FsReadResponse::FsReadValues(values)) => values,
			// A pinned read names an incarnation the first read found, so a missing one was
			// replaced or deleted meanwhile; the command retries instead of reporting no volume.
			Ok(FsReadResponse::FsReadError(FsReadError::NotFound))
			| Err(depot_client::filesystem::Error::Filesystem(depot_fs_core::Error::NotFound)) => {
				return Err(match pinned {
					Some(_) => WriterlessReadError::Unavailable,
					None => WriterlessReadError::NotFound,
				}
				.into());
			}
			Ok(FsReadResponse::FsReadError(FsReadError::TooLarge))
			| Err(depot_client::filesystem::Error::TooLarge) => {
				return Err(WriterlessReadError::TooLarge.into());
			}
			Ok(FsReadResponse::FsReadError(FsReadError::Overloaded))
			| Err(depot_client::filesystem::Error::Overloaded) => {
				return Err(WriterlessReadError::Overloaded.into());
			}
			Ok(_) | Err(_) => return Err(WriterlessReadError::Unavailable.into()),
		};
		let FsReadValues {
			volume_log_key,
			applied,
			as_of,
			committed,
			values,
		} = values;
		// A maintainer that does not know the commit, or a v1 or v2 answer upgraded on the way,
		// reports `u64::MAX`, and the lag is then unknown too: `u64::MAX`, never a distance.
		let lag = if committed == u64::MAX {
			u64::MAX
		} else {
			committed.saturating_sub(as_of)
		};
		// The volume has one claim log, so the applied cut names claim log 0 alone.
		let Ok(applied) = applied.into_single() else {
			tracing::debug!("point read answered with a cut over another claim log");
			return Err(WriterlessReadError::Unavailable.into());
		};
		if pinned
			.as_ref()
			.is_some_and(|pinned| pinned.volume_log_key != volume_log_key)
		{
			tracing::debug!("point read answered from another incarnation");
			return Err(WriterlessReadError::Unavailable.into());
		}
		if values.len() != keys.len() {
			tracing::debug!("point read returned the wrong number of values");
			return Err(WriterlessReadError::Unavailable.into());
		}
		// The snapshot is at `as_of`; when it is the applied position, that position names it.
		let position = match pinned {
			Some(pinned) => pinned.position,
			None if as_of == applied.volume_seq => applied,
			None => {
				tracing::debug!("latest point read answered below its applied position");
				return Err(WriterlessReadError::Unavailable.into());
			}
		};
		Ok((
			Pinned {
				volume_log_key,
				position,
			},
			values
				.into_iter()
				.map(|value| value.map(Vec::from))
				.collect(),
			lag,
		))
	}
}
