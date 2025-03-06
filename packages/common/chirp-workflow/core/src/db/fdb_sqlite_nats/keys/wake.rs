use std::{borrow::Cow, result::Result::Ok};

// TODO: Use concrete error types
use anyhow::*;
use foundationdb::tuple::{
	PackError, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::FormalKey;

#[derive(Debug)]
pub enum WakeCondition {
	Immediate,
	Deadline { deadline_ts: i64 },
	SubWorkflow { sub_workflow_id: Uuid },
	Signal { signal_id: Uuid },
	TaggedSignal { signal_id: Uuid },
}

#[derive(Debug)]
pub struct WorkflowWakeConditionKey {
	pub workflow_name: String,
	pub ts: i64,
	pub workflow_id: Uuid,
	pub condition: WakeCondition,
}

impl WorkflowWakeConditionKey {
	pub fn new(workflow_name: String, workflow_id: Uuid, condition: WakeCondition) -> Self {
		WorkflowWakeConditionKey {
			workflow_name,
			// NOTE: Override current ts with deadline
			ts: if let WakeCondition::Deadline { deadline_ts } = &condition {
				*deadline_ts
			} else {
				rivet_util::timestamp::now()
			},
			workflow_id,
			condition,
		}
	}

	pub fn subspace(workflow_name: String, ts: i64) -> WorkflowWakeConditionSubspaceKey {
		WorkflowWakeConditionSubspaceKey::new(workflow_name, ts)
	}

	pub fn subspace_without_ts(workflow_name: String) -> WorkflowWakeConditionSubspaceKey {
		WorkflowWakeConditionSubspaceKey::new_without_ts(workflow_name)
	}
}

impl FormalKey for WorkflowWakeConditionKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for WorkflowWakeConditionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		match &self.condition {
			WakeCondition::Immediate => {
				let t = (
					"wake",
					"workflow",
					&self.workflow_name,
					self.ts,
					self.workflow_id,
					"immediate",
				);
				t.pack(w, tuple_depth)
			}
			WakeCondition::Deadline { .. } => {
				let t = (
					"wake",
					"workflow",
					&self.workflow_name,
					// Already matches deadline ts, see `WorkflowWakeConditionKey::new`
					self.ts,
					self.workflow_id,
					"deadline",
				);
				t.pack(w, tuple_depth)
			}
			WakeCondition::SubWorkflow { sub_workflow_id } => {
				let t = (
					"wake",
					"workflow",
					&self.workflow_name,
					self.ts,
					self.workflow_id,
					"sub_workflow",
					sub_workflow_id,
				);
				t.pack(w, tuple_depth)
			}
			WakeCondition::Signal { signal_id } => {
				let t = (
					"wake",
					"workflow",
					&self.workflow_name,
					self.ts,
					self.workflow_id,
					"signal",
					signal_id,
				);
				t.pack(w, tuple_depth)
			}
			WakeCondition::TaggedSignal { signal_id } => {
				let t = (
					"wake",
					"workflow",
					&self.workflow_name,
					self.ts,
					self.workflow_id,
					"tagged_signal",
					signal_id,
				);
				t.pack(w, tuple_depth)
			}
		}
	}
}

impl<'de> TupleUnpack<'de> for WorkflowWakeConditionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_name, ts, workflow_id, condition_name)) =
			<(Cow<str>, Cow<str>, String, i64, Uuid, String)>::unpack(input, tuple_depth)?;

		let (input, v) = match &*condition_name {
			"immediate" => (
				input,
				WorkflowWakeConditionKey {
					workflow_name,
					ts,
					workflow_id,
					condition: WakeCondition::Immediate,
				},
			),
			"deadline" => (
				input,
				WorkflowWakeConditionKey {
					workflow_name,
					ts,
					workflow_id,
					condition: WakeCondition::Deadline {
						// See `WorkflowWakeConditionKey::new`
						deadline_ts: ts,
					},
				},
			),
			"sub_workflow" => {
				let (input, sub_workflow_id) = Uuid::unpack(input, tuple_depth)?;

				(
					input,
					WorkflowWakeConditionKey {
						workflow_name,
						ts,
						workflow_id,
						condition: WakeCondition::SubWorkflow { sub_workflow_id },
					},
				)
			}
			"signal" => {
				let (input, signal_id) = Uuid::unpack(input, tuple_depth)?;

				(
					input,
					WorkflowWakeConditionKey {
						workflow_name,
						ts,
						workflow_id,
						condition: WakeCondition::Signal { signal_id },
					},
				)
			}
			"tagged_signal" => {
				let (input, signal_id) = Uuid::unpack(input, tuple_depth)?;

				(
					input,
					WorkflowWakeConditionKey {
						workflow_name,
						ts,
						workflow_id,
						condition: WakeCondition::TaggedSignal { signal_id },
					},
				)
			}
			_ => {
				return Err(PackError::Message(
					format!("invalid wake condition type {condition_name:?} in key").into(),
				))
			}
		};

		Ok((input, v))
	}
}

// Structure should match `WorkflowWakeConditionKey`
pub struct WorkflowWakeConditionSubspaceKey {
	workflow_name: String,
	ts: Option<i64>,
}

impl WorkflowWakeConditionSubspaceKey {
	pub fn new(workflow_name: String, ts: i64) -> Self {
		WorkflowWakeConditionSubspaceKey {
			workflow_name,
			ts: Some(ts),
		}
	}

	pub fn new_without_ts(workflow_name: String) -> Self {
		WorkflowWakeConditionSubspaceKey {
			workflow_name,
			ts: None,
		}
	}
}

impl TuplePack for WorkflowWakeConditionSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = ("wake", "workflow", &self.workflow_name);
		offset += t.pack(w, tuple_depth)?;

		if let Some(ts) = &self.ts {
			offset += ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct TaggedSignalWakeKey {
	pub signal_name: String,
	/// For ordering.
	pub ts: i64,
	pub workflow_id: Uuid,
}

impl TaggedSignalWakeKey {
	pub fn new(signal_name: String, workflow_id: Uuid) -> Self {
		TaggedSignalWakeKey {
			signal_name,
			ts: rivet_util::timestamp::now(),
			workflow_id,
		}
	}

	pub fn subspace(signal_name: String) -> TaggedSignalWakeSubspaceKey {
		TaggedSignalWakeSubspaceKey::new(signal_name)
	}
}

impl FormalKey for TaggedSignalWakeKey {
	/// Workflow name and tags.
	type Value = TaggedSignalWakeData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for TaggedSignalWakeKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"wake",
			"tagged_signal",
			&self.signal_name,
			self.ts,
			self.workflow_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TaggedSignalWakeKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_name, ts, workflow_id)) =
			<(Cow<str>, Cow<str>, String, i64, Uuid)>::unpack(input, tuple_depth)?;
		let v = TaggedSignalWakeKey {
			signal_name,
			ts,
			workflow_id,
		};

		Ok((input, v))
	}
}

#[derive(Serialize, Deserialize)]
pub struct TaggedSignalWakeData {
	pub workflow_name: String,
	pub tags: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct TaggedSignalWakeSubspaceKey {
	signal_name: String,
}

impl TaggedSignalWakeSubspaceKey {
	pub fn new(signal_name: String) -> Self {
		TaggedSignalWakeSubspaceKey { signal_name }
	}
}

impl TuplePack for TaggedSignalWakeSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("wake", "tagged_signal", &self.signal_name);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct SubWorkflowWakeKey {
	pub sub_workflow_id: Uuid,
	/// For ordering.
	pub ts: i64,
	pub workflow_id: Uuid,
}

impl SubWorkflowWakeKey {
	pub fn new(sub_workflow_id: Uuid, workflow_id: Uuid) -> Self {
		SubWorkflowWakeKey {
			sub_workflow_id,
			ts: rivet_util::timestamp::now(),
			workflow_id,
		}
	}

	pub fn subspace(sub_workflow_id: Uuid) -> SubWorkflowWakeSubspaceKey {
		SubWorkflowWakeSubspaceKey::new(sub_workflow_id)
	}
}

impl FormalKey for SubWorkflowWakeKey {
	/// Workflow name (not sub workflow name).
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for SubWorkflowWakeKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"wake",
			"sub_workflow",
			&self.sub_workflow_id,
			self.ts,
			self.workflow_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SubWorkflowWakeKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, sub_workflow_id, ts, workflow_id)) =
			<(Cow<str>, Cow<str>, Uuid, i64, Uuid)>::unpack(input, tuple_depth)?;
		let v = SubWorkflowWakeKey {
			sub_workflow_id,
			ts,
			workflow_id,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct SubWorkflowWakeSubspaceKey {
	sub_workflow_id: Uuid,
}

impl SubWorkflowWakeSubspaceKey {
	pub fn new(sub_workflow_id: Uuid) -> Self {
		SubWorkflowWakeSubspaceKey { sub_workflow_id }
	}
}

impl TuplePack for SubWorkflowWakeSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("wake", "sub_workflow", self.sub_workflow_id);
		t.pack(w, tuple_depth)
	}
}
