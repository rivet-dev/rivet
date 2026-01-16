use anyhow::*;
use universaldb::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Metric {
	// Count (workflow name)
	WorkflowActive(String),
	/// Count (workflow name)
	WorkflowSleeping(String),
	/// Count (workflow name, error)
	WorkflowDead(String, String),
	/// Count (workflow name)
	WorkflowComplete(String),
	/// Deprecated
	SignalPending(String),
	/// Count (signal name)
	SignalPending2(String),
}

impl Metric {
	fn variant(&self) -> MetricVariant {
		match self {
			Metric::WorkflowActive(_) => MetricVariant::WorkflowActive,
			Metric::WorkflowSleeping(_) => MetricVariant::WorkflowSleeping,
			Metric::WorkflowDead(_, _) => MetricVariant::WorkflowDead,
			Metric::WorkflowComplete(_) => MetricVariant::WorkflowComplete,
			Metric::SignalPending(_) => MetricVariant::SignalPending,
			Metric::SignalPending2(_) => MetricVariant::SignalPending2,
		}
	}
}

#[derive(strum::FromRepr)]
enum MetricVariant {
	WorkflowActive = 0,
	WorkflowSleeping = 1,
	WorkflowDead = 2,
	WorkflowComplete = 3,
	// Deprecated
	SignalPending = 4,
	SignalPending2 = 5,
}

/// Stores gauge metrics for global database usage.
#[derive(Debug)]
pub struct MetricKey {
	pub metric: Metric,
}

impl MetricKey {
	pub fn new(metric: Metric) -> Self {
		MetricKey { metric }
	}

	pub fn subspace() -> MetricSubspaceKey {
		MetricSubspaceKey::new()
	}
}

impl FormalKey for MetricKey {
	// IMPORTANT: Uses LE bytes, not BE
	/// Count.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for MetricKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (METRIC, self.metric.variant() as i64);
		offset += t.pack(w, tuple_depth)?;

		offset += match &self.metric {
			Metric::WorkflowActive(workflow_name) => workflow_name.pack(w, tuple_depth)?,
			Metric::WorkflowSleeping(workflow_name) => workflow_name.pack(w, tuple_depth)?,
			Metric::WorkflowDead(workflow_name, error) => {
				(workflow_name, error).pack(w, tuple_depth)?
			}
			Metric::WorkflowComplete(workflow_name) => workflow_name.pack(w, tuple_depth)?,
			Metric::SignalPending(signal_name) => signal_name.pack(w, tuple_depth)?,
			Metric::SignalPending2(signal_name) => signal_name.pack(w, tuple_depth)?,
		};

		std::result::Result::Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for MetricKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, variant)) = <(usize, usize)>::unpack(input, tuple_depth)?;
		let variant = MetricVariant::from_repr(variant).ok_or_else(|| {
			PackError::Message(format!("invalid metric variant `{variant}` in key").into())
		})?;

		let (input, v) = match variant {
			MetricVariant::WorkflowActive => {
				let (input, workflow_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						metric: Metric::WorkflowActive(workflow_name),
					},
				)
			}
			MetricVariant::WorkflowSleeping => {
				let (input, workflow_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						metric: Metric::WorkflowSleeping(workflow_name),
					},
				)
			}
			MetricVariant::WorkflowDead => {
				let (input, (workflow_name, error)) =
					<(String, String)>::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						metric: Metric::WorkflowDead(workflow_name, error),
					},
				)
			}
			MetricVariant::WorkflowComplete => {
				let (input, workflow_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						metric: Metric::WorkflowComplete(workflow_name),
					},
				)
			}
			MetricVariant::SignalPending => {
				let (input, signal_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						metric: Metric::SignalPending(signal_name),
					},
				)
			}
			MetricVariant::SignalPending2 => {
				let (input, signal_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						metric: Metric::SignalPending2(signal_name),
					},
				)
			}
		};

		std::result::Result::Ok((input, v))
	}
}

/// Used to list all global gauge metrics.
pub struct MetricSubspaceKey {}

impl MetricSubspaceKey {
	pub fn new() -> Self {
		MetricSubspaceKey {}
	}
}

impl TuplePack for MetricSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (METRIC,);
		t.pack(w, tuple_depth)
	}
}
