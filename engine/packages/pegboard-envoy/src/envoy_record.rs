//! An Envoy connection's record. The record is the envoy's only durable state and its only
//! fencing record: every write compares the whole stored record by equality, so a late write from
//! a superseded connection can never overwrite a newer connection's record.
use anyhow::Result;
use pegboard::wire;

/// What a connection's record writes compare against and write next.
pub(crate) struct RecordState {
	/// The record this connection wants stored.
	pub entry: wire::EnvoyData,
	/// The stored record the next write replaces, or `None` for an absent record.
	pub expected: Option<wire::EnvoyData>,
	/// Whether this connection's first write landed. Later writes wait for it.
	pub connected: bool,
	/// The record this connection read before its first write, if it named another connection.
	pub previous: Option<wire::EnvoyData>,
}

/// A newer connection for the same envoy owns its record. This connection must close.
#[derive(Debug)]
pub(crate) struct Superseded;

impl std::fmt::Display for Superseded {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("envoy connection superseded")
	}
}

impl std::error::Error for Superseded {}

/// The pool rejected renewal of this connection's admission. It cannot be
/// renewed again; the runtime must reconnect and obtain a fresh admission.
#[derive(Debug)]
pub(crate) struct PoolAdmissionLost;

impl std::fmt::Display for PoolAdmissionLost {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("node pool admission is no longer valid")
	}
}

impl std::error::Error for PoolAdmissionLost {}

impl RecordState {
	/// The state of a new connection that read `stored` before connecting. The record keeps the
	/// envoy's creation time across connections and advances its connection epoch.
	pub(crate) fn new(
		stored: Option<wire::EnvoyData>,
		envoy_key: String,
		worker_id: Vec<u8>,
		connection_id: Vec<u8>,
		now_unix_ms: i64,
	) -> Self {
		let entry = wire::EnvoyData {
			key: envoy_key.into(),
			worker_id: worker_id.into(),
			connection_id: connection_id.clone().into(),
			connection_epoch: stored
				.as_ref()
				.map_or(1, |stored| stored.connection_epoch.saturating_add(1)),
			names: Vec::new(),
			create_unix_ms: stored
				.as_ref()
				.map_or(now_unix_ms, |stored| stored.create_unix_ms),
			heartbeat_unix_ms: now_unix_ms,
			stopping: false,
			connected: true,
		};
		Self {
			expected: stored.clone(),
			connected: false,
			previous: stored.filter(|stored| stored.connection_id.as_ref() != connection_id),
			entry,
		}
	}

	/// Whether the envoy had no record before this connection, so it is not in the listing
	/// index yet.
	pub(crate) fn is_new(&self) -> bool {
		self.expected.is_none()
	}

	/// Applies `change` as this connection's next record write, stamped with the time.
	pub(crate) fn next(&mut self, now_unix_ms: i64, change: impl FnOnce(&mut wire::EnvoyData)) {
		change(&mut self.entry);
		self.entry.heartbeat_unix_ms = now_unix_ms;
	}

	/// Resolves a write that found `stored` instead of the expected record. A record naming this
	/// connection is one of its own writes that landed after a lost reply, so it is adopted and
	/// the write retried. A record of an older connection, such as the close of the connection
	/// this one replaced landing after this one read the record, is written over. A record of a
	/// newer connection ends this one.
	pub(crate) fn resolve_mismatch(&mut self, stored: &wire::EnvoyData) -> Result<()> {
		if stored.connection_id == self.entry.connection_id
			|| stored.connection_epoch < self.entry.connection_epoch
		{
			self.expected = Some(stored.clone());
			return Ok(());
		}
		Err(Superseded.into())
	}

	/// Records a landed write.
	pub(crate) fn written(&mut self) {
		self.expected = Some(self.entry.clone());
		self.connected = true;
	}
}


