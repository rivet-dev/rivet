//! Encoding of `pegboard.actor` values that a process other than the writer reads: actor KV
//! values, start notices and keyed actor reservations. Each is sealed at the version its
//! readers read, the fleet-selected write version or the receiving worker's version, never
//! at the compiled latest, so a process of the previous release still decodes it while a
//! deploy rolls.

use anyhow::Result;
use bytes::Bytes;
use rivet_data::families::VersionFamily;
use rivet_topology::{
	versions::{PeerVersions, WriteVersions},
	worker::WorkerId,
};
use vbare::Versioned;

/// The version family of every value this module seals.
pub const FAMILY: VersionFamily = VersionFamily::PEGBOARD_ACTOR;

/// Which value is sealed: the `value` label of `pegboard_actor_values_sealed_total`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SealedValue {
	/// An actor KV value, read by the process serving the actor's next start.
	Kv,
	/// A keyed actor's Epoxy reservation, read by any process resolving the key.
	Reservation,
	/// A start notice to a waiting gateway.
	StartNotice,
}

impl SealedValue {
	fn label(self) -> &'static str {
		match self {
			Self::Kv => "kv",
			Self::Reservation => "reservation",
			Self::StartNotice => "start_notice",
		}
	}
}

/// Seals `value` at `version`. A value `version` cannot carry is refused before it is
/// written or sent with the retryable `depot.overloaded` reason `version_pending` and
/// `retry_after_ms`, so a released client sends the request again once the view shows the
/// readers upgraded. Any other encoding failure is an error.
pub fn seal<W: Versioned>(
	kind: SealedValue,
	value: &W::Latest,
	version: u16,
	retry_after_ms: u64,
) -> Result<Bytes> {
	match W::seal_to_bytes(value, version) {
		Ok(bytes) => {
			crate::metrics::ACTOR_VALUES_SEALED
				.with_label_values(&[kind.label(), &version.to_string()])
				.inc();
			Ok(bytes)
		}
		Err(vbare::EncodeError::Conversion(error)) if version < W::LATEST => {
			tracing::debug!(
				?error,
				value = kind.label(),
				version,
				"refused a pegboard.actor value the version its readers read cannot carry; a reader runs an older release"
			);
			Err(depot_client::client_protocol::version_pending(
				retry_after_ms,
			))
		}
		Err(error) => Err(error.into()),
	}
}

/// Seals a value that any process may read later, such as an actor KV value or a keyed actor
/// reservation, at `versions`' write version of [`FAMILY`]: the highest every live storage
/// worker reads.
pub fn seal_stored<W: Versioned>(
	kind: SealedValue,
	value: &W::Latest,
	versions: &dyn WriteVersions,
	retry_after_ms: u64,
) -> Result<Bytes> {
	let version = versions.write_version(FAMILY)?;
	seal::<W>(kind, value, version, retry_after_ms)
}

/// Seals a message to `worker`, which decodes, acts on and drops it, at the version topology's
/// in-memory view says `worker` reads. See [`PeerVersions::peer_version`].
pub fn seal_for_worker<W: Versioned>(
	kind: SealedValue,
	value: &W::Latest,
	versions: &dyn PeerVersions,
	worker: &WorkerId,
) -> Result<Bytes> {
	let version = versions.peer_version(FAMILY, worker)?;
	seal::<W>(kind, value, version, versions.refresh_hint_ms())
}
