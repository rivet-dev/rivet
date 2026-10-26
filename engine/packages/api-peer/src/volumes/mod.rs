//! Stateless filesystem HTTP reads. Snapshot authority and retention live in the maintainer.
mod address;
mod read;

use address::{Address, Kind};
use axum::{
	Json, Router,
	body::Body,
	extract::{OriginalUri, State},
	response::{IntoResponse, Response},
	routing::get,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use depot_fs_core::{
	CHUNK_BYTES, FsDirectoryCursor, FsKind,
	store::{Row, RowKey},
};
use http::{HeaderMap, StatusCode};
use read::Snapshot;
use rivet_config::config::api_peer::VolumesConfig;
use rivet_data::{
	generated::{shared_depot_fs_read_v1::*, shared_depot_types_v1::VolumeLogKey},
	versioned::DepotFsDirectoryCursorData,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use vbare::Versioned;

type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
enum Error {
	Invalid,
	NotFound,
	TooOld,
	TooLarge,
	Unavailable,
	/// The resolved client's scope ended, so retrying through it cannot succeed.
	Revoked,
	BaseUnavailable,
	Overloaded,
	Lagging,
	Corrupt,
	Range(u64),
}
impl Error {
	fn retryable(self) -> bool {
		matches!(
			self,
			Self::TooOld
				| Self::Unavailable
				| Self::BaseUnavailable
				| Self::Overloaded
				| Self::Lagging
		)
	}
	fn code(self) -> &'static str {
		match self {
			Self::Invalid => "Invalid",
			Self::NotFound => "NotFound",
			Self::TooOld => "TooOld",
			Self::TooLarge => "TooLarge",
			Self::Unavailable | Self::Revoked => "Unavailable",
			Self::BaseUnavailable => "BaseUnavailable",
			Self::Overloaded => "Overloaded",
			Self::Lagging => "Lagging",
			Self::Corrupt => "Corrupt",
			Self::Range(_) => "RangeNotSatisfiable",
		}
	}
}
impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.code())
	}
}
impl std::error::Error for Error {}
impl From<FsReadError> for Error {
	fn from(error: FsReadError) -> Self {
		match error {
			FsReadError::TooOld => Self::TooOld,
			FsReadError::Lagging => Self::Lagging,
			FsReadError::TooLarge => Self::TooLarge,
			FsReadError::Overloaded => Self::Overloaded,
			FsReadError::NotFound => Self::NotFound,
			FsReadError::Corrupt => Self::Corrupt,
			FsReadError::BaseUnavailable => Self::BaseUnavailable,
			FsReadError::Fenced | FsReadError::Unavailable | FsReadError::NotServing => {
				Self::Unavailable
			}
		}
	}
}
impl From<depot_fs_core::Error> for Error {
	fn from(error: depot_fs_core::Error) -> Self {
		use depot_fs_core::Error as Core;
		match error {
			Core::NotFound => Self::NotFound,
			Core::TooLarge => Self::TooLarge,
			Core::Corrupt | Core::ReadRequired(_) => Self::Corrupt,
			_ => Self::Invalid,
		}
	}
}
impl IntoResponse for Error {
	fn into_response(self) -> Response {
		let status = match self {
			Self::Invalid => StatusCode::BAD_REQUEST,
			Self::NotFound => StatusCode::NOT_FOUND,
			Self::TooOld => StatusCode::CONFLICT,
			Self::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
			Self::Unavailable
			| Self::Revoked
			| Self::BaseUnavailable
			| Self::Lagging
			| Self::Overloaded => StatusCode::SERVICE_UNAVAILABLE,
			Self::Corrupt => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Range(_) => StatusCode::RANGE_NOT_SATISFIABLE,
		};
		let mut response =
			(status, Json(serde_json::json!({"error": self.code()}))).into_response();
		if let Self::Range(size) = self
			&& let Ok(value) = format!("bytes */{size}").parse()
		{
			response
				.headers_mut()
				.insert(http::header::CONTENT_RANGE, value);
		}
		response
	}
}
#[derive(Clone)]
struct HttpState {
	client: Arc<dyn Fn() -> depot_client::Client + Send + Sync>,
	config: VolumesConfig,
	requests: Arc<Semaphore>,
}

/// Construction starts no tasks and retains no read sessions between HTTP requests.
pub fn router(
	client: Arc<dyn Fn() -> depot_client::Client + Send + Sync>,
	config: VolumesConfig,
) -> anyhow::Result<Router> {
	config.validate()?;
	anyhow::ensure!(
		config.max_requests <= Semaphore::MAX_PERMITS,
		"filesystem HTTP request limit exceeds semaphore capacity"
	);
	let state = HttpState {
		client,
		requests: Arc::new(Semaphore::new(config.max_requests)),
		config,
	};
	Ok(Router::new()
		.route("/volumes/{partition}/{name}/{kind}", get(handle))
		.route("/volumes/{partition}/{name}/{kind}/", get(handle))
		.route("/volumes/{partition}/{name}/{kind}/{*path}", get(handle))
		.with_state(state))
}

async fn pause(client: &depot_client::Client, deadline: u64) -> Result<()> {
	let clock = client.env().clock();
	let now = clock.observe().map_err(|_| Error::Unavailable)?.boot_ms;
	if now >= deadline {
		return Err(Error::Unavailable);
	}
	clock
		.sleep_until(now.saturating_add(20).min(deadline))
		.await
		.map_err(|_| Error::Unavailable)
}
async fn handle(
	State(state): State<HttpState>,
	OriginalUri(uri): OriginalUri,
	headers: HeaderMap,
) -> Result<Response> {
	let permit = Arc::new(
		state
			.requests
			.clone()
			.try_acquire_owned()
			.map_err(|_| Error::Overloaded)?,
	);
	let address = Address::parse(&uri, state.config.max_path_bytes)?;
	let cursor = address
		.query
		.get(b"cursor".as_slice())
		.map(|encoded| {
			let bytes = URL_SAFE_NO_PAD
				.decode(encoded)
				.map_err(|_| Error::Invalid)?;
			DepotFsDirectoryCursorData::deserialize_with_embedded_version(&bytes)
				.map_err(|_| Error::Invalid)
		})
		.transpose()?;
	let clock = (state.client)().env().clock();
	let deadline = rivet_env::deadline_after(
		clock.as_ref(),
		Duration::from_millis(state.config.request_timeout_ms),
	)
	.map_err(|_| Error::Unavailable)?;
	loop {
		// Resolve the installed client on every attempt, so a request that arrives while Depot
		// starts is served as soon as Depot is ready instead of retrying an unready client
		// until its deadline.
		let client = (state.client)();
		let snapshot = match &cursor {
			Some(cursor) => Ok(Snapshot {
				client: client.clone(),
				key: VolumeLogKey {
					volume_key: address.volume.clone(),
					volume_incarnation: cursor.incarnation,
				},
				session: cursor.session,
				as_of: cursor.applied_seq,
				deadline,
			}),
			None => Snapshot::latest(client.clone(), address.volume.clone(), deadline).await,
		};
		let snapshot = match snapshot {
			Ok(snapshot) => snapshot,
			Err(error) if error.retryable() => {
				pause(&client, deadline).await?;
				continue;
			}
			Err(error) => return Err(error),
		};
		let result = serve(
			&state,
			&address,
			&headers,
			cursor.as_ref(),
			&snapshot,
			permit.clone(),
		)
		.await;
		match result {
			Ok(mut response) => {
				response.headers_mut().insert(
					http::header::CACHE_CONTROL,
					http::HeaderValue::from_static("no-store"),
				);
				return Ok(response);
			}
			Err(error) => {
				if cursor.is_none() || !error.retryable() || matches!(error, Error::TooOld) {
					snapshot.close().await;
				}
				if cursor.is_some() && matches!(error, Error::TooOld | Error::NotFound) {
					return Err(Error::TooOld);
				}
				if !error.retryable() || matches!(error, Error::TooOld) && cursor.is_some() {
					return Err(error);
				}
				pause(&client, deadline).await?;
			}
		}
	}
}
async fn serve(
	state: &HttpState,
	address: &Address,
	headers: &HeaderMap,
	cursor: Option<&FsDirectoryCursor>,
	snapshot: &Snapshot,
	permit: Arc<OwnedSemaphorePermit>,
) -> Result<Response> {
	let (ino, inode) = snapshot.resolve(&address.path).await?;
	match address.kind {
		Kind::Stat => {
			snapshot.close().await;
			Ok(Json(serde_json::json!({"inode": ino, "attributes": inode})).into_response())
		}
		Kind::Kv => {
			if inode.kind != FsKind::Kv {
				return Err(Error::Invalid);
			}
			let key = address.query.get(b"key".as_slice()).ok_or(Error::Invalid)?;
			let Some(Row::Bytes(bytes)) = snapshot.get(RowKey::Kv(ino, key.clone())).await? else {
				return Err(Error::NotFound);
			};
			snapshot.close().await;
			Ok((
				[(http::header::CONTENT_TYPE, "application/octet-stream")],
				bytes,
			)
				.into_response())
		}
		Kind::Dirs => {
			if inode.kind != FsKind::Dir || cursor.is_some_and(|c| c.directory != ino) {
				return Err(Error::Invalid);
			}
			let after = cursor.map(|c| RowKey::Dentry(ino, c.last_name.clone().to_vec()));
			let rows = snapshot
				.rows(FsReadOperation::FsReadScan(FsReadScan {
					at: snapshot.at(),
					range: FsRowRange::FsDirectoryRows(FsDirectoryRows { ino }),
					after: after.clone().map(Into::into),
					limit: state.config.directory_page_size,
				}))
				.await?;
			if rows.len() > state.config.directory_page_size as usize {
				return Err(Error::Corrupt);
			}
			let mut entries = Vec::with_capacity(rows.len());
			let mut last = after;
			for (key, row) in rows {
				if last.as_ref().is_some_and(|previous| *previous >= key) {
					return Err(Error::Corrupt);
				}
				match (&key, row) {
					(RowKey::Dentry(parent, name), Row::Dentry(entry)) if *parent == ino => {
						entries.push(
							serde_json::json!({"name": hex::encode(name), "inode": entry.child, "kind": entry.kind}),
						);
					}
					_ => return Err(Error::Corrupt),
				}
				last = Some(key);
			}
			let next = if entries.len() == state.config.directory_page_size as usize {
				let Some(RowKey::Dentry(_, last_name)) = last else {
					return Err(Error::Corrupt);
				};
				let cursor = FsDirectoryCursor {
					incarnation: snapshot.key.volume_incarnation,
					session: snapshot.session,
					directory: ino,
					applied_seq: snapshot.as_of,
					last_name: last_name.into(),
				};
				Some(
					URL_SAFE_NO_PAD.encode(
						DepotFsDirectoryCursorData::serialize_with_embedded_version(&cursor, 1)
							.map_err(|_| Error::Corrupt)?,
					),
				)
			} else {
				snapshot.close().await;
				None
			};
			Ok(Json(serde_json::json!({"entries": entries, "cursor": next})).into_response())
		}
		Kind::Files => {
			if inode.kind != FsKind::File {
				return Err(Error::Invalid);
			}
			let etag = format!(
				"\"{}-{}-{ino}\"",
				hex::encode(snapshot.key.volume_incarnation),
				snapshot.as_of
			);
			// An unknown date or a stale/weak validator requires the complete body.
			let range_allowed = headers
				.get(http::header::IF_RANGE)
				.is_none_or(|value| value.as_bytes() == etag.as_bytes());
			let range = headers
				.get(http::header::RANGE)
				.filter(|_| range_allowed)
				.map(|value| value.to_str().map_err(|_| Error::Range(inode.size)))
				.transpose()?;
			let (start, end) = address::range(range, inode.size)?;
			let mut response = http::Response::builder()
				.status(if range.is_some() {
					StatusCode::PARTIAL_CONTENT
				} else {
					StatusCode::OK
				})
				.header(http::header::CONTENT_TYPE, "application/octet-stream")
				.header(http::header::CONTENT_LENGTH, end - start)
				.header(http::header::ACCEPT_RANGES, "bytes")
				.header(http::header::ETAG, etag);
			if range.is_some() {
				response = response.header(
					http::header::CONTENT_RANGE,
					format!("bytes {start}-{}/{size}", end - 1, size = inode.size),
				);
			}
			let body = if let Some(bytes) = inode.inline {
				if bytes.len() as u64 != inode.size {
					return Err(Error::Corrupt);
				}
				snapshot.close().await;
				Body::from(bytes[start as usize..end as usize].to_vec())
			} else {
				let stream = futures_util::stream::try_unfold(
					(snapshot.clone(), start, permit),
					move |(snapshot, offset, permit)| async move {
						if offset == end {
							snapshot.close().await;
							return Ok(None);
						}
						let index = offset / CHUNK_BYTES as u64;
						let rows = loop {
							match snapshot
								.rows(FsReadOperation::FsReadChunks(FsReadChunks {
									at: snapshot.at(),
									ino,
									ranges: vec![FsChunkRange {
										first: index,
										count: 1,
									}],
								}))
								.await
							{
								Ok(rows) => break rows,
								Err(error)
									if error.retryable() && !matches!(error, Error::TooOld) =>
								{
									pause(&snapshot.client, snapshot.deadline).await?
								}
								Err(error) => return Err(error),
							}
						};
						if rows.len() > 1 {
							return Err(Error::Corrupt);
						}
						let mut bytes = vec![0; CHUNK_BYTES];
						match rows.into_iter().next() {
							Some((key, Row::Bytes(value)))
								if key == RowKey::Chunk(ino, index)
									&& value.len() <= CHUNK_BYTES =>
							{
								bytes[..value.len()].copy_from_slice(&value);
							}
							Some(_) => return Err(Error::Corrupt),
							None => {}
						}
						let within = (offset % CHUNK_BYTES as u64) as usize;
						let length = (end - offset).min((CHUNK_BYTES - within) as u64) as usize;
						Ok::<_, Error>(Some((
							Bytes::copy_from_slice(&bytes[within..within + length]),
							(snapshot, offset + length as u64, permit),
						)))
					},
				);
				Body::from_stream(stream)
			};
			response.body(body).map_err(|_| Error::Corrupt)
		}
	}
}


