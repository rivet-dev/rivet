//! Epoxy consensus for a single datacenter.
//!
//! Every key's voting scope is this datacenter's own replica, so a key's acceptor record is
//! the whole quorum: a proposal is one accept on the key's volume, which the key machine commits
//! in the same transition, and every read is a read of that record. No proposal state, no
//! ballot rounds and no remote replica are involved.
//!
//! **Multi-region contract.** The key's persisted acceptor record (`epoxy.key_state.v1.bare`)
//! keeps the full single-decree Paxos shape: a phase (empty, committed, promised or accepted),
//! ballots, a version per value and the key's recorded voting scope. A single-datacenter
//! proposal is a fast-ballot accept with the scope `[local replica]`, so every record it writes
//! is a valid multi-region record whose scope names this replica alone. A multi-region
//! proposer adopts such a key as it is: the recorded scope tells it the key's quorum, and the
//! committed version is the predecessor of its next instance. Changing the record's layout
//! needs a new schema version and a converter; adding proposers needs neither.

use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{Result, bail, ensure};
use epoxy_protocol::protocol;

use crate::{depot::replica::Replica, key_machine::types::FAST_COUNTER, metrics};

pub type ResponseFuture<'a> = Pin<Box<dyn Future<Output = Result<protocol::Response>> + Send + 'a>>;

/// Sends the existing Epoxy peer protocol to a regional replica.
pub trait ReplicaTransport: Send + Sync {
	/// Gate a worker-local proposal with the same capability check as peer requests.
	fn admit(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
		Box::pin(async { Ok(()) })
	}

	fn request(&self, request: protocol::Request) -> ResponseFuture<'_>;
}

#[derive(Clone)]
pub struct Consensus {
	local: Replica,
	transport: Arc<dyn ReplicaTransport>,
}

#[derive(Clone)]
pub enum Command {
	Set {
		key: Vec<u8>,
		value: Option<Vec<u8>>,
	},
	CheckAndSet {
		key: Vec<u8>,
		expect_one_of: Vec<Option<Vec<u8>>>,
		value: Option<Vec<u8>>,
	},
}

#[derive(Clone)]
pub struct Proposal {
	pub command: Command,
	pub mutable: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProposalResult {
	Committed,
	ExpectedValueDoesNotMatch(Option<Vec<u8>>),
}

/// The key's open instance holds another proposal's promise or accepted value, which only a
/// multi-region proposer leaves behind. A single-datacenter proposal never overrides it.
#[derive(Debug)]
pub struct Contended;

impl std::fmt::Display for Contended {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Epoxy key holds an unfinished proposal from another proposer")
	}
}

impl std::error::Error for Contended {}

impl Consensus {
	pub fn new(local: Replica, transport: Arc<dyn ReplicaTransport>) -> Self {
		Self { local, transport }
	}

	/// Proposes `proposal` as one accept on the key's volume. The accept carries a fast ballot
	/// derived from the command, so a retry of the same command is the same accept and finds
	/// its value committed. A mutable key's next version is chosen by the key machine.
	#[tracing::instrument(skip_all, fields(replica_id = self.local.replica_id()))]
	pub async fn propose(&self, proposal: &Proposal) -> Result<ProposalResult> {
		self.transport.admit().await?;
		let (key, value, expect_one_of) = match &proposal.command {
			Command::Set { key, value } => (key, value, None),
			Command::CheckAndSet {
				key,
				expect_one_of,
				value,
			} => {
				ensure!(
					!expect_one_of.is_empty(),
					"Epoxy compare-and-set needs an expected value"
				);
				(key, value, Some(expect_one_of))
			}
		};
		let request = protocol::AcceptRequest {
			key: key.clone().into(),
			value: value.clone().map(Into::into),
			ballot: protocol::Ballot {
				counter: FAST_COUNTER,
				replica_id: self.local.replica_id(),
				proposal_incarnation: fast_incarnation(
					key,
					value.as_deref(),
					expect_one_of.map(Vec::as_slice),
					proposal.mutable,
				),
			},
			mutable: proposal.mutable,
			version: 1,
			expect_one_of: expect_one_of.map(|expected| {
				expected
					.iter()
					.map(|value| value.clone().map(Into::into))
					.collect()
			}),
			scope: vec![self.local.replica_id()],
			predecessor: None,
		};
		let result = metrics::observe("propose", "local", self.local.accept(&request)).await;
		let outcome = result.and_then(|reply| decide(reply, value.as_deref()));
		record_proposal(&outcome);
		outcome
	}

	/// The key's committed value on this replica, which is the key's only voting replica.
	#[tracing::instrument(skip_all, fields(replica_id = self.local.replica_id()))]
	pub async fn get(&self, key: &[u8]) -> Result<Option<protocol::CommittedValue>> {
		metrics::observe("read", "local", self.local.get(key)).await
	}

	/// The key's acceptor state on this replica, including a value accepted but not committed.
	pub async fn get_owner(
		&self,
		key: &[u8],
		replica_id: u64,
	) -> Result<protocol::KvReadStateResponse> {
		ensure!(
			replica_id == self.local.replica_id(),
			"Epoxy owner reads address this datacenter's replica only"
		);
		metrics::observe("read", "owner", self.local.inspect(key)).await
	}
}

/// Reads the key machine's answer to a single-datacenter accept.
fn decide(reply: protocol::AcceptResponse, value: Option<&[u8]>) -> Result<ProposalResult> {
	use protocol::AcceptResponse as Reply;
	Ok(match reply {
		Reply::AcceptResponseCommitted(_) => ProposalResult::Committed,
		// A decided instance whose value is the one proposed is this command, already applied
		// by an earlier attempt or by an identical concurrent one.
		Reply::AcceptResponseAlreadyCommitted(decided)
			if decided.committed.value.as_deref() == value =>
		{
			ProposalResult::Committed
		}
		Reply::AcceptResponseAlreadyCommitted(decided) => {
			ProposalResult::ExpectedValueDoesNotMatch(decided.committed.value.map(Into::into))
		}
		Reply::AcceptResponseMismatch(mismatch) => ProposalResult::ExpectedValueDoesNotMatch(
			mismatch
				.committed
				.and_then(|committed| committed.value.map(Into::into)),
		),
		Reply::AcceptResponseContended(_) => return Err(Contended.into()),
		Reply::AcceptResponseOk(_) => {
			bail!("Epoxy key accepted a single-datacenter value without committing it")
		}
		Reply::AcceptResponseHigherBallot(_) => {
			bail!("Epoxy key refused a fast ballot with a higher one")
		}
		Reply::AcceptResponseScopeMismatch(mismatch) => {
			bail!("Epoxy key is scoped to other regions {:?}", mismatch.scope)
		}
	})
}

/// Counts one answered proposal.
fn record_proposal(outcome: &Result<ProposalResult>) {
	let result = match outcome {
		Ok(ProposalResult::Committed) => "committed",
		Ok(ProposalResult::ExpectedValueDoesNotMatch(_)) => "mismatch",
		Err(error) if error.downcast_ref::<Contended>().is_some() => "contended",
		Err(_) => "error",
	};
	metrics::PROPOSAL_TOTAL
		.with_label_values(&["local", result])
		.inc();
}

/// The fast ballot's incarnation: a hash of the whole command, so the same command always
/// carries the same ballot and a retry is idempotent at the acceptor.
pub fn fast_incarnation(
	key: &[u8],
	value: Option<&[u8]>,
	expect_one_of: Option<&[Option<Vec<u8>>]>,
	mutable: bool,
) -> u64 {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(b"rivet.epoxy.proposal_incarnation.v2");
	let mut part = |bytes: Option<&[u8]>| match bytes {
		Some(bytes) => {
			hasher.update([1]);
			hasher.update((bytes.len() as u64).to_le_bytes());
			hasher.update(bytes);
		}
		None => hasher.update([0]),
	};
	part(Some(key));
	part(value);
	match expect_one_of {
		Some(expected) => {
			part(Some(&(expected.len() as u64).to_le_bytes()));
			for value in expected {
				part(value.as_deref());
			}
		}
		None => part(None),
	}
	part(Some(&[u8::from(mutable)]));
	let digest = hasher.finalize();
	let mut bytes = [0; 8];
	bytes.copy_from_slice(&digest[..8]);
	u64::from_le_bytes(bytes)
}
