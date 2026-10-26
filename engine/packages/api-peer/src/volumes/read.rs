//! Request-scoped, deadline-bound snapshot reads through the shared Depot client.
use super::{Error, Result};
use depot_client::Client;
use depot_fs_core::{
	FsInode, ROOT_INO, rules,
	store::{ReadRequest, Row, RowKey, RowRange, RowStore},
};
use rivet_data::generated::{
	shared_depot_fs_read_v1::*,
	shared_depot_types_v1::{VolumeKey, VolumeLogKey},
};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct Snapshot {
	pub client: Client,
	pub key: VolumeLogKey,
	pub session: [u8; 16],
	pub as_of: u64,
	pub deadline: u64,
}

async fn request(client: &Client, request: FsReadRequest, deadline: u64) -> Result<FsReadResponse> {
	let clock = client.env().clock();
	let result = rivet_env::timeout_at(
		clock.as_ref(),
		deadline,
		client.read_filesystem(request, deadline),
	)
	.await
	.map_err(|_| Error::Unavailable)?
	.map_err(|error| {
		if depot_client::ScopeRevoked::is(&error) {
			Error::Revoked
		} else {
			Error::Unavailable
		}
	})?;
	match result {
		FsReadResponse::FsReadError(error) => Err(error.into()),
		other => Ok(other),
	}
}
impl Snapshot {
	pub async fn latest(client: Client, key: VolumeKey, deadline: u64) -> Result<Self> {
		let response = request(
			&client,
			FsReadRequest {
				target: FsReadTarget::VolumeKey(key.clone()),
				client_epoch: None,
				operation: FsReadOperation::FsReadOpen(FsReadOpen {
					position: None,
					prefetch: None,
				}),
			},
			deadline,
		)
		.await?;
		let FsReadResponse::FsReadOpened(opened) = response else {
			return Err(Error::Corrupt);
		};
		if opened.volume_log_key.volume_key != key || opened.as_of != opened.applied.volume_seq {
			return Err(Error::Corrupt);
		}
		Ok(Self {
			client,
			key: opened.volume_log_key,
			session: opened.session.ok_or(Error::Corrupt)?.id,
			as_of: opened.as_of,
			deadline,
		})
	}
	pub async fn close(&self) {
		// If the HTTP caller disconnects or its deadline has elapsed, the original
		// maintainer TTL still bounds the hold. Closing never renews a session.
		let _ = request(
			&self.client,
			FsReadRequest {
				target: FsReadTarget::VolumeLogKey(self.key.clone()),
				client_epoch: None,
				operation: FsReadOperation::FsReadClose(FsReadClose {
					session: self.session,
				}),
			},
			self.deadline,
		)
		.await;
	}
	pub async fn rows(&self, operation: FsReadOperation) -> Result<Vec<(RowKey, Row)>> {
		let response = request(
			&self.client,
			FsReadRequest {
				target: FsReadTarget::VolumeLogKey(self.key.clone()),
				client_epoch: None,
				operation,
			},
			self.deadline,
		)
		.await?;
		let FsReadResponse::FsReadRows(rows) = response else {
			return Err(Error::Corrupt);
		};
		if rows.session != Some(self.session)
			|| rows.as_of != self.as_of
			|| rows.applied.volume_seq < self.as_of
		{
			return Err(Error::Corrupt);
		}
		rows.rows
			.into_iter()
			.map(|row| depot_fs_core::wire::decode_row(row).map_err(|_| Error::Corrupt))
			.collect()
	}
	/// Where this snapshot's row reads read: through its session.
	pub fn at(&self) -> FsReadAt {
		FsReadAt::FsSessionRead(FsSessionRead {
			session: self.session,
		})
	}
	pub async fn get(&self, key: RowKey) -> Result<Option<Row>> {
		let mut rows = self
			.rows(FsReadOperation::FsReadGet(FsReadGet {
				at: self.at(),
				key: key.clone().into(),
			}))
			.await?;
		if rows.len() > 1 || rows.first().is_some_and(|(found, _)| *found != key) {
			return Err(Error::Corrupt);
		}
		Ok(rows.pop().map(|(_, row)| row))
	}
	pub async fn resolve(&self, path: &[u8]) -> Result<(u64, FsInode)> {
		let mut cache = Resolver::default();
		let mut bytes = 0usize;
		loop {
			let resolved = rules::resolve(&cache, ROOT_INO, path, true, None).and_then(|ino| {
				depot_fs_core::store::inode(&cache, ino, None).map(|inode| (ino, inode))
			});
			match resolved {
				Ok(found) => return Ok(found),
				Err(depot_fs_core::Error::ReadRequired(ReadRequest::Get(key))) => {
					let row = self.get(key.clone()).await?;
					bytes = bytes.saturating_add(
						512 + match &row {
							Some(Row::Inode(inode)) => {
								inode.target.len()
									+ inode.inline.as_ref().map_or(0, |inline| inline.len())
							}
							_ => 0,
						},
					);
					if bytes > 1024 * 1024 || cache.0.len() >= 4096 {
						return Err(Error::TooLarge);
					}
					cache.0.insert(key, row);
				}
				Err(error) => return Err(error.into()),
			}
		}
	}
}
#[derive(Default)]
struct Resolver(BTreeMap<RowKey, Option<Row>>);
impl RowStore for Resolver {
	fn get(&self, key: &RowKey, _: Option<u64>) -> depot_fs_core::Result<Option<Row>> {
		self.0
			.get(key)
			.cloned()
			.ok_or_else(|| depot_fs_core::Error::ReadRequired(ReadRequest::Get(key.clone())))
	}
	fn scan(
		&self,
		_: &RowRange,
		_: Option<&RowKey>,
		_: usize,
		_: Option<u64>,
	) -> depot_fs_core::Result<Vec<(RowKey, Row)>> {
		Err(depot_fs_core::Error::Corrupt)
	}
}
