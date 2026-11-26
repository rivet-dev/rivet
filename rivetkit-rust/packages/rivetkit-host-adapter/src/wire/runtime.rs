//! The handle table and the two directions of byte traffic.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow};
use bytes::Bytes;
use parking_lot::Mutex;
use rivet_error::{RivetError, RivetErrorKind};
use rivetkit_core::websocket::WebSocket;
use rivetkit_core::{ActorContext, KeepAwakeRegion, SqliteTransaction};
use tokio::sync::{OwnedSemaphorePermit, oneshot};
use tokio_util::sync::CancellationToken;

use super::messages::{ErrorBody, decode, encode};
use super::{Event, EventKind, Status, Transport};
use crate::errors::rivet_error_with_meta;

/// Bodies larger than this are leased instead of lent.
pub const LEASE_THRESHOLD: usize = 64 * 1024;

/// A reply or completion: header and body.
#[derive(Debug)]
pub(crate) struct Answer {
	pub header: Vec<u8>,
	pub body: Vec<u8>,
}

impl Answer {
	pub fn header(header: Vec<u8>) -> Self {
		Self {
			header,
			body: Vec::new(),
		}
	}

	pub fn empty() -> Self {
		Self::header(Vec::new())
	}
}

pub(crate) struct ActorSlot {
	pub ctx: ActorContext,
}

pub(crate) enum Region {
	KeepAwake(#[allow(dead_code)] Box<KeepAwakeRegion>),
	/// Dropping the sender ends the `wait_until` future.
	WaitUntil(#[allow(dead_code)] oneshot::Sender<()>),
}

/// Everything a `u64` handle can name.
pub(crate) enum Handle {
	Registry(Arc<super::registry::RegistryHandle>),
	Actor(Arc<ActorSlot>),
	/// A host reply Rust is waiting for.
	Event(oneshot::Sender<Result<Answer>>),
	Lease {
		#[allow(dead_code)]
		bytes: Bytes,
		/// Returned to the stream's credit window on release.
		#[allow(dead_code)]
		credit: Option<OwnedSemaphorePermit>,
	},
	Region(#[allow(dead_code)] Region),
	Tx(Arc<SqliteTransaction>),
	ResponseBody(Arc<super::body::OrderedBody>),
	/// An inbound stream; releasing it cancels the pump.
	InStream(CancellationToken),
	WebSocket(WebSocket),
	Client(Arc<rivetkit_client::Client>),
	ClientConn(Arc<super::client::ConnSlot>),
	ClientSocket(Arc<super::client::SocketSlot>),
}

struct Entry {
	/// The actor handle this belongs to, or 0. Swept when the actor stops.
	owner: u64,
	handle: Handle,
}

/// One per process (or per test). Owns the handle table and talks to the host
/// through a [`Transport`].
pub struct Wire {
	transport: Box<dyn Transport>,
	pub(crate) tokio: tokio::runtime::Handle,
	next_id: AtomicU64,
	handles: Mutex<HashMap<u64, Entry>>,
}

impl Wire {
	pub fn new(transport: Box<dyn Transport>, tokio: tokio::runtime::Handle) -> Arc<Self> {
		Arc::new(Self {
			transport,
			tokio,
			next_id: AtomicU64::new(1),
			handles: Mutex::new(HashMap::new()),
		})
	}

	pub(crate) fn insert(&self, owner: u64, handle: Handle) -> u64 {
		let id = self.next_id.fetch_add(1, Ordering::Relaxed);
		self.handles.lock().insert(id, Entry { owner, handle });
		id
	}

	pub(crate) fn remove(&self, id: u64) -> Option<Handle> {
		self.handles.lock().remove(&id).map(|entry| entry.handle)
	}

	/// Looks up `id` and maps it with `f`, which returns `None` for the wrong kind.
	pub(crate) fn get<T>(
		&self,
		id: u64,
		kind: &str,
		f: impl FnOnce(&Handle) -> Option<T>,
	) -> Result<T> {
		self.handles
			.lock()
			.get(&id)
			.and_then(|entry| f(&entry.handle))
			.ok_or_else(|| anyhow!("unknown or released {kind} handle {id}"))
	}

	pub(crate) fn actor(&self, id: u64) -> Result<Arc<ActorSlot>> {
		self.get(id, "actor", |handle| match handle {
			Handle::Actor(slot) => Some(slot.clone()),
			_ => None,
		})
	}

	/// The owner of a handle (for handles created on behalf of an actor).
	pub(crate) fn owner_of(&self, id: u64) -> u64 {
		self.handles.lock().get(&id).map_or(0, |entry| entry.owner)
	}

	/// Removes every handle `owner` created. Called when an actor stops, and when a
	/// client is released (its connections, sockets and bodies go with it).
	pub(crate) fn release_owned(&self, owner: u64) {
		let removed: Vec<(u64, Handle)> = {
			let mut handles = self.handles.lock();
			let ids: Vec<u64> = handles
				.iter()
				// Leases stay until the host releases them: the host may still be
				// reading the buffer in place after the actor stopped.
				.filter(|(_, entry)| {
					entry.owner == owner && !matches!(entry.handle, Handle::Lease { .. })
				})
				.map(|(id, _)| *id)
				.collect();
			ids.into_iter()
				.filter_map(|id| handles.remove(&id).map(|entry| (id, entry.handle)))
				.collect()
		};
		// Dropped outside the lock: some handles run code on drop.
		for (id, handle) in removed {
			self.cleanup_tree(id, handle);
		}
	}

	/// Cleans up `handle` and, for a client, everything it owns.
	fn cleanup_tree(&self, id: u64, handle: Handle) {
		let owns_handles = matches!(handle, Handle::Client(_));
		self.cleanup(handle);
		if owns_handles {
			self.release_owned(id);
		}
	}

	/// Delivers an event. Small bodies are lent; large ones are leased.
	pub(crate) fn emit(
		&self,
		kind: EventKind,
		actor_type: u64,
		actor: u64,
		id: u64,
		header: &[u8],
		body: Vec<u8>,
	) {
		if body.len() <= LEASE_THRESHOLD {
			self.transport.event(Event {
				kind,
				actor_type,
				actor,
				id,
				header,
				body: &body,
				lease: 0,
			});
		} else {
			self.emit_leased(kind, actor_type, actor, id, header, Bytes::from(body), None);
		}
	}

	/// Delivers an event whose body stays alive until the host releases the lease.
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn emit_leased(
		&self,
		kind: EventKind,
		actor_type: u64,
		actor: u64,
		id: u64,
		header: &[u8],
		body: Bytes,
		credit: Option<OwnedSemaphorePermit>,
	) {
		// The table keeps `body`'s heap buffer alive and unmoved; this clone shares it.
		let view = body.clone();
		let lease = self.insert(
			actor,
			Handle::Lease {
				bytes: body,
				credit,
			},
		);
		self.transport.event(Event {
			kind,
			actor_type,
			actor,
			id,
			header,
			body: &view,
			lease,
		});
	}

	/// Completes a host call.
	pub(crate) fn complete(&self, token: u64, result: Result<Answer>) {
		match result {
			Ok(answer) if answer.body.len() > LEASE_THRESHOLD => {
				let body = Bytes::from(answer.body);
				let view = body.clone();
				let lease = self.insert(
					0,
					Handle::Lease {
						bytes: body,
						credit: None,
					},
				);
				self.transport
					.complete(token, Status::Ok, &answer.header, &view, lease);
			}
			Ok(answer) => {
				self.transport
					.complete(token, Status::Ok, &answer.header, &answer.body, 0)
			}
			Err(error) => {
				let header = encode(&error_body(&error));
				self.transport
					.complete(token, Status::Error, &header, &[], 0);
			}
		}
	}

	/// Completes `token` with `error`.
	pub fn fail(&self, token: u64, error: anyhow::Error) {
		self.complete(token, Err(error));
	}

	/// Answers a pending event (`rivet_reply`).
	pub fn reply(&self, event: u64, status: i32, header: &[u8], body: &[u8]) -> Result<()> {
		let tx = {
			let mut handles = self.handles.lock();
			match handles.get(&event) {
				Some(Entry {
					handle: Handle::Event(_),
					..
				}) => match handles.remove(&event) {
					Some(Entry {
						handle: Handle::Event(tx),
						..
					}) => tx,
					_ => unreachable!("checked under the same lock"),
				},
				// Unknown, already answered, or cancelled.
				_ => return Err(anyhow!("unknown or released event handle {event}")),
			}
		};
		let result = if status == Status::Ok as i32 {
			Ok(Answer {
				header: header.to_vec(),
				body: body.to_vec(),
			})
		} else {
			Err(error_from_header(header))
		};
		let _ = tx.send(result);
		Ok(())
	}

	/// Frees a leased buffer and returns its stream credit.
	pub fn lease_release(&self, lease: u64) -> Result<()> {
		let mut handles = self.handles.lock();
		match handles.get(&lease) {
			Some(Entry {
				handle: Handle::Lease { .. },
				..
			}) => {
				handles.remove(&lease);
				Ok(())
			}
			_ => Err(anyhow!("unknown or released lease {lease}")),
		}
	}

	/// Releases a handle the host owns. Actors and events belong to Rust and are
	/// left alone.
	pub fn release(&self, id: u64) -> Result<()> {
		let handle = {
			let mut handles = self.handles.lock();
			let releasable = handles
				.get(&id)
				.is_some_and(|entry| !matches!(entry.handle, Handle::Actor(_) | Handle::Event(_)));
			if !releasable {
				return Err(anyhow!("handle {id} is unknown or not releasable"));
			}
			handles.remove(&id).map(|entry| entry.handle)
		};
		if let Some(handle) = handle {
			self.cleanup_tree(id, handle);
		}
		Ok(())
	}

	/// Sends a no-reply event about the actor.
	pub(crate) fn notify(
		&self,
		kind: EventKind,
		actor_type: u64,
		actor: u64,
		id: u64,
		header: &[u8],
	) {
		self.transport.event(Event {
			kind,
			actor_type,
			actor,
			id,
			header,
			body: &[],
			lease: 0,
		});
	}

	/// Forwards a log record to the host as a `LOG` event.
	pub fn log(&self, level: &str, target: &str, message: String) {
		let header = encode(&super::messages::LogEvent {
			level: level.to_owned(),
			target: target.to_owned(),
			message,
		});
		self.notify(EventKind::Log, 0, 0, 0, &header);
	}

	/// Spawns `work` on the runtime and completes `token` with its result. A panic in
	/// `work` still completes the token (as an internal error): the host may be
	/// blocked on it with no timeout.
	pub(crate) fn spawn_complete(
		self: &Arc<Self>,
		token: u64,
		work: impl std::future::Future<Output = Result<Answer>> + Send + 'static,
	) {
		let pending = self.pending(token);
		self.tokio.spawn(async move {
			let result = work.await;
			pending.complete(result);
		});
	}

	/// A token that completes exactly once: through [`Pending::complete`], or with an
	/// error when dropped first (the task holding it panicked or returned early).
	pub(crate) fn pending(self: &Arc<Self>, token: u64) -> Pending {
		Pending {
			wire: self.clone(),
			token: Some(token),
		}
	}

	#[cfg(test)]
	pub(crate) fn handle_count(&self) -> usize {
		self.handles.lock().len()
	}
}

/// See [`Wire::pending`].
pub(crate) struct Pending {
	wire: Arc<Wire>,
	token: Option<u64>,
}

impl Pending {
	pub(crate) fn complete(mut self, result: Result<Answer>) {
		if let Some(token) = self.token.take() {
			self.wire.complete(token, result);
		}
	}
}

impl Drop for Pending {
	fn drop(&mut self) {
		if let Some(token) = self.token.take() {
			let reason = if std::thread::panicking() {
				"operation panicked"
			} else {
				"operation ended without a result"
			};
			self.wire.complete(token, Err(anyhow!(reason)));
		}
	}
}

impl Wire {
	/// Side effects of letting go of a handle. Runs on any thread.
	fn cleanup(&self, handle: Handle) {
		match handle {
			Handle::Registry(registry) => registry.released(self),
			Handle::InStream(cancel) => cancel.cancel(),
			// Dropping it ends its queue; its task aborts the body if the host never ended it.
			Handle::ResponseBody(_) => {}
			Handle::ClientConn(conn) => conn.close(),
			Handle::ClientSocket(socket) => socket.close(),
			Handle::Tx(tx) => {
				self.tokio.spawn(async move {
					let _ = tx.rollback().await;
				});
			}
			_ => {}
		}
	}
}

/// Rivet's `{group, code, message, metadata}` for any error.
pub(crate) fn error_body(error: &anyhow::Error) -> ErrorBody {
	// Errors the client got back from an actor keep the actor's group and code.
	if let Some(action) = error.downcast_ref::<rivetkit_client::ActionError>() {
		return ErrorBody {
			group: action.group.clone(),
			code: action.code.clone(),
			message: action.message.clone(),
			metadata: action.metadata.clone(),
		};
	}
	let extracted = RivetError::extract(error);
	let message = match extracted.kind {
		// Errors without a schema surface as internal errors whose message may be
		// hidden. The host still needs to see what went wrong.
		RivetErrorKind::Static(schema)
			if schema.group == "core" && schema.code == "internal_error" =>
		{
			format!("{error:#}")
		}
		_ => extracted.message().to_owned(),
	};
	let metadata = extracted
		.meta
		.as_ref()
		.and_then(|raw| serde_json::from_str(raw.get()).ok());
	ErrorBody {
		group: extracted.group().to_owned(),
		code: extracted.code().to_owned(),
		message,
		metadata,
	}
}

/// A host error header as a Rivet error, so clients see the host's group, code
/// and metadata.
pub(crate) fn error_from_header(header: &[u8]) -> anyhow::Error {
	match decode::<ErrorBody>(header, "error body") {
		Ok(body) => rivet_error_with_meta(&body.group, &body.code, body.message, body.metadata),
		Err(error) => error,
	}
}
