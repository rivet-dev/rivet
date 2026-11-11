//! Individual actor forks: the lineage a forked actor carries in `_rivet_meta`, and the one run
//! of the fork hook at the first start of each fork.
//!
//! The engine names the fork on every start of a forked actor. The copied database holds its
//! source's `appliedForkId`, so the first start of the copy sees a fork it has not applied: the
//! start runs the runtime's fork hook, appends the fork to the chain and records it as applied,
//! all in the startup commit. A crash before that commit leaves the copy untouched, and the
//! next start runs the hook again.
//!
//! The hook's state edits and the applied fork are one commit on every SQLite backend, as long
//! as the hook leaves them to the startup save. On native SQLite its SQL writes, and any save
//! it starts itself, are held in that commit too; a save still in flight at the startup save is
//! waited for, so the commit carries the hook's final state. On remote SQLite (wasm runtimes)
//! SQL writes and state transactions the hook commits itself land at once, without the fork
//! record, so a crash or a failed start before the fork is recorded runs the hook again on top
//! of them, and the hook must be idempotent there. A save the hook only requests is folded into
//! the startup save. On native SQLite the hook's writes must fit one commit (`commit_page_cap`):
//! a transaction that would take them past it fails as too large instead of being committed
//! early by the VFS, so nothing the hook wrote lands without the fork record. A new actor's held
//! startup writes differ: the VFS commits those held so far early instead.

use anyhow::{Context, Result};
use rivet_envoy_client::protocol;
use rivetkit_actor_persist::generated::fork_v1;
use rivetkit_actor_persist::versioned as persist_versioned;

use crate::actor::persist::{
	decode_latest_with_embedded_version, encode_latest_with_embedded_version,
};
use crate::types::ActorKey;

/// The `_rivet_meta` key of the actor's fork state.
pub(crate) const FORK_STATE_META_KEY: &str = "fork_state";

const FORK_STATE_VERSION: u16 = 1;

/// One individual actor fork behind an actor: the source and when the fork was made.
#[derive(Clone, Debug, PartialEq)]
pub struct ForkLink {
	pub actor_id: String,
	pub key: ActorKey,
	/// Milliseconds since the epoch.
	pub forked_at: i64,
}

/// A fork whose hook runs in this start: the link it adds to the chain and the fork input.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorForkStart {
	pub link: ForkLink,
	pub input: Option<Vec<u8>>,
}

/// The fork a start command names, before startup knows whether its hook already ran.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StartFork {
	fork_id: Vec<u8>,
	link: fork_v1::ForkLink,
	/// The source's own chain as the engine recorded it, oldest first, without this fork.
	source_chain: Vec<fork_v1::ForkLink>,
}

impl StartFork {
	pub(crate) fn from_protocol(fork: protocol::ActorFork) -> Self {
		Self {
			fork_id: fork.fork_id.to_vec(),
			link: fork_v1::ForkLink {
				actor_id: fork.actor_id.to_string().into(),
				key: fork.key.map(|key| key.to_string().into()),
				forked_at: fork.forked_at,
			},
			source_chain: fork
				.chain
				.into_iter()
				.map(|link| fork_v1::ForkLink {
					actor_id: link.actor_id.to_string().into(),
					key: link.key.map(|key| key.to_string().into()),
					forked_at: link.forked_at,
				})
				.collect(),
		}
	}
}

/// The actor's fork state as stored, empty for an actor that was never forked.
pub(crate) fn empty_state() -> fork_v1::ForkState {
	fork_v1::ForkState {
		applied_fork_id: None,
		chain: Vec::new(),
	}
}

pub(crate) fn decode_state(bytes: &[u8]) -> Result<fork_v1::ForkState> {
	decode_latest_with_embedded_version::<persist_versioned::ForkState>(bytes, "fork state")
}

pub(crate) fn encode_state(state: &fork_v1::ForkState) -> Result<Vec<u8>> {
	encode_latest_with_embedded_version::<persist_versioned::ForkState>(
		state.clone(),
		FORK_STATE_VERSION,
		"fork state",
	)
}

/// The fork `start` names when this database has not applied it, as the state to persist once
/// its hook ran and the start the runtime receives. `None` when the start names no fork or the
/// hook already ran for it.
pub(crate) fn pending(
	state: &fork_v1::ForkState,
	start: Option<StartFork>,
	input: Option<Vec<u8>>,
) -> Option<(fork_v1::ForkState, ActorForkStart)> {
	let start = start?;
	let applied = state
		.applied_fork_id
		.as_ref()
		.is_some_and(|applied| applied.as_ref() as &[u8] == start.fork_id.as_slice());
	if applied {
		return None;
	}

	let mut next = state.clone();
	next.applied_fork_id = Some(start.fork_id.into());
	next.chain = source_chain(&state.chain, start.source_chain);
	next.chain.push(start.link.clone());
	Some((
		next,
		ActorForkStart {
			link: link_of(&start.link),
			input,
		},
	))
}

/// The source's chain that this fork extends. The copied chain is the source's when it ends with
/// the same link as the engine's: the source committed its first start before it was copied.
/// Otherwise the copy holds an older chain, or none, because the source was forked again before
/// its startup commit landed or ran under an SDK that never recorded its chain, and the engine's
/// chain replaces it. An empty engine chain, from a source whose record predates engine chains,
/// keeps the copied one.
fn source_chain(
	copied: &[fork_v1::ForkLink],
	engine: Vec<fork_v1::ForkLink>,
) -> Vec<fork_v1::ForkLink> {
	let Some(engine_last) = engine.last() else {
		return copied.to_vec();
	};
	if copied.last() == Some(engine_last) {
		return copied.to_vec();
	}
	engine
}

/// The chain the runtime reads, oldest first.
pub(crate) fn chain_of(state: &fork_v1::ForkState) -> Vec<ForkLink> {
	state.chain.iter().map(link_of).collect()
}

fn link_of(stored: &fork_v1::ForkLink) -> ForkLink {
	ForkLink {
		actor_id: stored.actor_id.to_string(),
		key: crate::registry::actor_key_from_protocol(stored.key.as_ref().map(ToString::to_string)),
		forked_at: stored.forked_at,
	}
}

/// Reads the fork state a startup row carries, empty when the actor was never forked.
pub(crate) fn state_from_row(bytes: Option<Vec<u8>>) -> Result<fork_v1::ForkState> {
	match bytes {
		Some(bytes) => decode_state(&bytes).context("decode actor fork state"),
		None => Ok(empty_state()),
	}
}
