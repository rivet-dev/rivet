use crate::{client::Client, consensus};
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct Proposal {
	pub commands: Vec<Command>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct Command {
	pub kind: CommandKind,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub enum CommandKind {
	SetCommand(SetCommand),
	CheckAndSetCommand(CheckAndSetCommand),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct SetCommand {
	pub key: Vec<u8>,
	pub value: Option<Vec<u8>>,
}

impl std::fmt::Debug for SetCommand {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SetCommand")
			.field("key_bytes", &self.key.len())
			.field("value", &self.value.as_ref().map(|_| "<redacted>"))
			.finish()
	}
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct CheckAndSetCommand {
	pub key: Vec<u8>,
	/// Checked against the value the proposal would replace.
	pub expect_one_of: Vec<Option<Vec<u8>>>,
	pub new_value: Option<Vec<u8>>,
}

impl std::fmt::Debug for CheckAndSetCommand {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CheckAndSetCommand")
			.field("key_bytes", &self.key.len())
			.field("expected_count", &self.expect_one_of.len())
			.field("new_value", &self.new_value.as_ref().map(|_| "<redacted>"))
			.finish()
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProposalResult {
	Committed,
	ConsensusFailed { reason: ConsensusFailedReason },
}

impl ProposalResult {
	/// Errors if the result is not `Committed`
	pub fn resolve(&self) -> Result<()> {
		match self {
			ProposalResult::Committed => Ok(()),
			ProposalResult::ConsensusFailed { reason } => match reason {
				ConsensusFailedReason::ExpectedValueDoesNotMatch { .. } => {
					bail!("proposal failed due to value mismatch")
				}
			},
		}
	}
}

#[derive(Serialize, Deserialize)]
pub enum ConsensusFailedReason {
	ExpectedValueDoesNotMatch { current_value: Option<Vec<u8>> },
}

impl std::fmt::Debug for ConsensusFailedReason {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ExpectedValueDoesNotMatch { .. } => f
				.debug_struct("ExpectedValueDoesNotMatch")
				.field("current_value", &"<redacted>")
				.finish(),
		}
	}
}

#[derive(Debug)]
pub struct Input {
	pub proposal: Proposal,
	pub mutable: bool,
}

/// Runs the proposal on this datacenter's replica, the key's only voting replica.
pub(crate) async fn execute(client: &Client, input: &Input) -> Result<ProposalResult> {
	ensure!(
		input.proposal.commands.len() == 1,
		"Epoxy only supports single-command proposals"
	);
	let command = match &input.proposal.commands[0].kind {
		CommandKind::SetCommand(command) => consensus::Command::Set {
			key: command.key.clone(),
			value: command.value.clone(),
		},
		CommandKind::CheckAndSetCommand(command) => consensus::Command::CheckAndSet {
			key: command.key.clone(),
			expect_one_of: command.expect_one_of.clone(),
			value: command.new_value.clone(),
		},
	};
	let result = client
		.consensus()
		.propose(&consensus::Proposal {
			command,
			mutable: input.mutable,
		})
		.await?;
	Ok(match result {
		consensus::ProposalResult::Committed => ProposalResult::Committed,
		consensus::ProposalResult::ExpectedValueDoesNotMatch(current_value) => {
			ProposalResult::ConsensusFailed {
				reason: ConsensusFailedReason::ExpectedValueDoesNotMatch { current_value },
			}
		}
	})
}
