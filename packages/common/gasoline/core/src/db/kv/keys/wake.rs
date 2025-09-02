use std::result::Result::Ok;

use anyhow::*;
use rivet_util::Id;
use udb_util::prelude::*;

#[derive(Debug)]
pub enum WakeCondition {
	Immediate,
	Deadline { deadline_ts: i64 },
	SubWorkflow { sub_workflow_id: Id },
	Signal { signal_id: Id },
}

impl WakeCondition {
	fn variant(&self) -> WakeConditionVariant {
		match self {
			WakeCondition::Immediate => WakeConditionVariant::Immediate,
			WakeCondition::Deadline { .. } => WakeConditionVariant::Deadline,
			WakeCondition::SubWorkflow { .. } => WakeConditionVariant::SubWorkflow,
			WakeCondition::Signal { .. } => WakeConditionVariant::Signal,
		}
	}

	pub fn deadline_ts(&self) -> Option<i64> {
		match self {
			WakeCondition::Deadline { deadline_ts } => Some(*deadline_ts),
			_ => None,
		}
	}
}

#[derive(strum::FromRepr)]
enum WakeConditionVariant {
	Immediate = 0,
	Deadline = 1,
	SubWorkflow = 2,
	Signal = 3,
}

#[derive(Debug)]
pub struct WorkflowWakeConditionKey {
	pub workflow_name: String,
	pub ts: i64,
	pub workflow_id: Id,
	pub condition: WakeCondition,
}

impl WorkflowWakeConditionKey {
	pub fn new(workflow_name: String, workflow_id: Id, condition: WakeCondition) -> Self {
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
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			WAKE,
			WORKFLOW,
			&self.workflow_name,
			self.ts,
			self.workflow_id,
			self.condition.variant() as usize,
		);
		offset += t.pack(w, tuple_depth)?;

		match &self.condition {
			WakeCondition::Immediate | WakeCondition::Deadline { .. } => {}
			WakeCondition::SubWorkflow { sub_workflow_id } => {
				offset += sub_workflow_id.pack(w, tuple_depth)?;
			}
			WakeCondition::Signal { signal_id } => {
				offset += signal_id.pack(w, tuple_depth)?;
			}
		}

		Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowWakeConditionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_name, ts, workflow_id, wake_condition_variant)) =
			<(usize, usize, String, i64, Id, usize)>::unpack(input, tuple_depth)?;
		let wake_condition_variant = WakeConditionVariant::from_repr(wake_condition_variant)
			.ok_or_else(|| {
				PackError::Message(
					format!("invalid wake condition variant `{wake_condition_variant}` in key")
						.into(),
				)
			})?;

		let (input, v) = match wake_condition_variant {
			WakeConditionVariant::Immediate => (
				input,
				WorkflowWakeConditionKey {
					workflow_name,
					ts,
					workflow_id,
					condition: WakeCondition::Immediate,
				},
			),
			WakeConditionVariant::Deadline => (
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
			WakeConditionVariant::SubWorkflow => {
				let (input, sub_workflow_id) = Id::unpack(input, tuple_depth)?;

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
			WakeConditionVariant::Signal => {
				let (input, signal_id) = Id::unpack(input, tuple_depth)?;

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

		let t = (WAKE, WORKFLOW, &self.workflow_name);
		offset += t.pack(w, tuple_depth)?;

		if let Some(ts) = &self.ts {
			offset += ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct SubWorkflowWakeKey {
	pub sub_workflow_id: Id,
	/// For ordering.
	pub ts: i64,
	pub workflow_id: Id,
}

impl SubWorkflowWakeKey {
	pub fn new(sub_workflow_id: Id, workflow_id: Id) -> Self {
		SubWorkflowWakeKey {
			sub_workflow_id,
			ts: rivet_util::timestamp::now(),
			workflow_id,
		}
	}

	pub fn subspace(sub_workflow_id: Id) -> SubWorkflowWakeSubspaceKey {
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
			WAKE,
			SUB_WORKFLOW,
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
			<(usize, usize, Id, i64, Id)>::unpack(input, tuple_depth)?;
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
	sub_workflow_id: Id,
}

impl SubWorkflowWakeSubspaceKey {
	pub fn new(sub_workflow_id: Id) -> Self {
		SubWorkflowWakeSubspaceKey { sub_workflow_id }
	}
}

impl TuplePack for SubWorkflowWakeSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WAKE, SUB_WORKFLOW, self.sub_workflow_id);
		t.pack(w, tuple_depth)
	}
}
