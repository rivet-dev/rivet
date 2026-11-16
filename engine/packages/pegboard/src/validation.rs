//! Edge validation of names that end up inside formal keys. Actor names, actor keys and pool
//! names sit inside partition keys, which Depot caps in size, so every create path and the
//! Envoy connect check them before anything is written.
use anyhow::Result;
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

/// The largest actor name, in bytes.
pub const MAX_ACTOR_NAME_BYTES: usize = 64;
/// The largest actor key, in bytes.
pub const MAX_ACTOR_KEY_BYTES: usize = 1024;
pub const MAX_NAMESPACE_NAME_BYTES: usize = 63;
/// The largest pool name, in bytes.
pub const MAX_POOL_NAME_BYTES: usize = 63;

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"invalid_name",
	"Actor name is invalid.",
	"Actor name must be 1 to {max_bytes} bytes."
)]
pub struct InvalidActorName {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"invalid_key",
	"Actor key is invalid.",
	"Actor key must be at most {max_bytes} bytes."
)]
pub struct InvalidActorKey {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"envoy",
	"invalid_namespace_name",
	"Namespace name is invalid.",
	"Namespace name must be a DNS label of 1 to {max_bytes} lowercase letters, digits and hyphens."
)]
pub struct InvalidNamespaceName {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"envoy",
	"invalid_pool_name",
	"Pool name is invalid.",
	"Pool name must be a DNS label of 1 to {max_bytes} lowercase letters, digits and hyphens."
)]
pub struct InvalidPoolName {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError)]
#[error("namespace", "not_found", "The namespace does not exist.")]
pub struct NamespaceNotFound;

/// The namespace refuses the work while it waits on something that does not finish on its own
/// soon: its fork, or the fleet's upgrade to namespace volumes. A namespace whose group create
/// is still settling answers [`NamespaceCreating`] instead.
#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"pending",
	"The namespace's creation or fork has not finished; retry."
)]
pub struct NamespacePending;

/// The namespace's group create has not settled yet, for example a create whose first attempt's
/// outcome is unknown until the namespace's own retry finishes it. Nothing was done; retry after
/// `retry_after_ms`, which the edge does.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"namespace",
	"creating",
	"The namespace is still being created; retry.",
	"The namespace is still being created; retry after {retry_after_ms} ms."
)]
pub struct NamespaceCreating {
	pub retry_after_ms: u64,
}

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"archived",
	"The namespace is archived or being archived."
)]
pub struct NamespaceArchived;

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"failed",
	"The namespace's fork was refused and its name is retired."
)]
pub struct NamespaceFailed;

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"not_forkable",
	"The namespace cannot be forked until its volumes from before volume groups are moved into its group."
)]
pub struct NamespaceNotForkable;

/// A migration from V2 lands actors in this namespace, so a fork's child would hold only the
/// actors already moved: its fork reset destroys the copy of every actor the migration still holds
/// or has delegated to V2. Refused until the migration ends and its config is removed.
#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"migrating_from_v2",
	"The namespace cannot be forked while a V2 migration moves actors into it."
)]
pub struct NamespaceMigratingFromV2;

/// Legacy: the namespace still has volumes from before namespaces had groups, so it cannot be
/// archived until they are copied into its group. Removed in 3.1 with the backfill.
#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"backfill_pending",
	"The namespace's volumes from before volume groups are still being moved into its group; retry later."
)]
pub struct NamespaceBackfillPending;

/// Legacy: the namespace's finished backfill gave up on actors from before namespaces had groups,
/// which stay at unprefixed keys outside its group, so it cannot be forked or archived until an
/// operator destroys them. Removed in 3.1 with the backfill.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"namespace",
	"backfill_skipped",
	"The namespace still has actors from before volume groups that its backfill could not move; destroy them first.",
	"The namespace still has {skipped_count} actors from before volume groups that its backfill could not move; destroy each by its ID or key first."
)]
pub struct NamespaceBackfillSkipped {
	pub skipped_count: u32,
}

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"lineage_too_deep",
	"The namespace is as many forks below its root as the lineage depth cap allows."
)]
pub struct NamespaceLineageTooDeep;

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"has_forks",
	"A fork of this namespace is not archived; archive it first."
)]
pub struct NamespaceHasForks;

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"has_nodes",
	"A node of this namespace is connected; stop its nodes before archiving it."
)]
pub struct NamespaceHasNodes;

#[derive(Debug, RivetError)]
#[error("namespace", "protected", "The default namespace cannot be archived.")]
pub struct NamespaceProtected;

#[derive(Debug, RivetError)]
#[error(
	"namespace",
	"name_not_unique",
	"A namespace with this name already exists."
)]
pub struct NamespaceNameNotUnique;

pub fn actor_name(name: &str) -> Result<()> {
	if name.is_empty() || name.len() > MAX_ACTOR_NAME_BYTES {
		return Err(InvalidActorName {
			max_bytes: MAX_ACTOR_NAME_BYTES,
		}
		.build());
	}
	Ok(())
}

pub fn actor_key(key: Option<&str>) -> Result<()> {
	if key.is_some_and(|key| key.len() > MAX_ACTOR_KEY_BYTES) {
		return Err(InvalidActorKey {
			max_bytes: MAX_ACTOR_KEY_BYTES,
		}
		.build());
	}
	Ok(())
}

pub fn namespace_name(namespace: &str) -> Result<()> {
	let valid = !namespace.is_empty()
		&& namespace.len() <= MAX_NAMESPACE_NAME_BYTES
		&& namespace
			.bytes()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		&& !namespace.starts_with('-')
		&& !namespace.ends_with('-');
	if !valid {
		return Err(InvalidNamespaceName {
			max_bytes: MAX_NAMESPACE_NAME_BYTES,
		}
		.build());
	}
	Ok(())
}

pub fn pool_name(pool: &str) -> Result<()> {
	let valid = !pool.is_empty()
		&& pool.len() <= MAX_POOL_NAME_BYTES
		&& pool
			.bytes()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		&& !pool.starts_with('-')
		&& !pool.ends_with('-');
	if !valid {
		return Err(InvalidPoolName {
			max_bytes: MAX_POOL_NAME_BYTES,
		}
		.build());
	}
	Ok(())
}

#[derive(Debug, RivetError)]
#[error("pool", "not_found", "The node pool does not exist.")]
pub struct PoolNotFound;

/// An individual actor fork that failed for good: its database copy failed and the new actor
/// was destroyed with its key released, or the source never started and nothing was created.
/// `reason` says which.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"fork_failed",
	"The actor fork failed.",
	"The actor fork failed: {reason}"
)]
pub struct ActorForkFailed {
	pub reason: String,
}

impl ActorForkFailed {
	/// The database copy failed for good and the new actor was destroyed.
	pub fn copy_failed() -> Self {
		Self {
			reason: "the database copy failed and the new actor was destroyed.".to_owned(),
		}
	}

	/// The source has no database volume to copy, because it never started.
	pub fn source_never_started() -> Self {
		Self {
			reason: "the source actor never started, so it has no database to copy; nothing was created.".to_owned(),
		}
	}
}

/// An individual actor fork refused at admission while an engine worker from before actor forks
/// may run: the start command that carries the fork would reach such a worker without it. The
/// fork did nothing; retry after `retry_after_ms`.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"fork_unavailable",
	"Actor forks are unavailable until the engine upgrade finishes; retry.",
	"Actor forks are unavailable until every engine worker supports them; retry after {retry_after_ms} ms."
)]
pub struct ActorForkUnavailable {
	pub retry_after_ms: u64,
}

/// Legacy: a wake of an actor that the backfill is moving into its namespace's group, from
/// before namespaces had groups; the new behavior has no such move. Nothing was started; retry
/// after `retry_after_ms`, which resolves the actor again and finds it at its new id once the
/// move made it. Answered only while the namespace's backfill is not done. Removed in 3.1 with
/// the backfill.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"moving",
	"The actor is being moved by an engine upgrade. Clients never receive this code: the gateway retries the request, then answers guard.service_unavailable with reason actor_moving and a Retry-After hint.",
	"The actor is being moved by an engine upgrade; retry after {retry_after_ms} ms."
)]
pub struct LegacyActorMoving {
	pub retry_after_ms: u64,
}

/// Legacy: an Envoy event sent to a copy the backfill made of the legacy actor with the same id
/// and abandoned ([`crate::legacy_volumes::legacy_abandoned_copy`]). The copy is not the actor, so
/// it refuses the event instead of accepting and ignoring it, and Envoy event delivery sends the
/// event to the legacy actor's unprefixed volume. The new behavior has no such copy. Answered only
/// to Envoy event delivery, inside the engine. Removed in 3.1 with the backfill.
#[derive(Debug, RivetError)]
#[error(
	"actor",
	"legacy_moved_away",
	"The actor event reached an abandoned copy of an actor being moved by an engine upgrade. Clients never receive this code: the engine delivers the event to the original actor."
)]
pub struct LegacyActorMovedAway;

/// An individual actor fork whose request deadline passed after the new actor was created,
/// while its database copy goes on. The fork is not retried: read the actor `actor_id` later
/// to see whether it started or was destroyed.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"fork_pending",
	"The actor fork is still in progress; read the new actor later.",
	"The actor fork is still in progress; read actor {actor_id} later."
)]
pub struct ActorForkPending {
	pub actor_id: String,
}

/// Actor or fork input that is not valid base64.
#[derive(Debug, RivetError)]
#[error("actor", "invalid_input", "The input is not valid base64.")]
pub struct ActorInvalidInput;

/// An individual actor fork of a source whose database is already at Depot's fork chain depth
/// limit. The new actor was destroyed and its key released.
#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"fork_too_deep",
	"The actor is too many forks deep to fork again.",
	"The actor's database is already {max_depth} forks deep."
)]
pub struct ActorForkTooDeep {
	pub max_depth: usize,
}
