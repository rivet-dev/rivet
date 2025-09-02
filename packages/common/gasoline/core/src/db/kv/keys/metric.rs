use anyhow::*;
use udb_util::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub enum GaugeMetric {
	WorkflowActive(String),
	WorkflowSleeping(String),
	WorkflowDead(String, String),
	WorkflowComplete(String),
	SignalPending(String),
}

impl GaugeMetric {
	fn variant(&self) -> GaugeMetricVariant {
		match self {
			GaugeMetric::WorkflowActive(_) => GaugeMetricVariant::WorkflowActive,
			GaugeMetric::WorkflowSleeping(_) => GaugeMetricVariant::WorkflowSleeping,
			GaugeMetric::WorkflowDead(_, _) => GaugeMetricVariant::WorkflowDead,
			GaugeMetric::WorkflowComplete(_) => GaugeMetricVariant::WorkflowComplete,
			GaugeMetric::SignalPending(_) => GaugeMetricVariant::SignalPending,
		}
	}
}

#[derive(strum::FromRepr)]
enum GaugeMetricVariant {
	WorkflowActive = 0,
	WorkflowSleeping = 1,
	WorkflowDead = 2,
	WorkflowComplete = 3,
	SignalPending = 4,
}

#[derive(Debug)]
pub struct GaugeMetricKey {
	pub metric: GaugeMetric,
}

impl GaugeMetricKey {
	pub fn new(metric: GaugeMetric) -> Self {
		GaugeMetricKey { metric }
	}

	pub fn subspace() -> GaugeMetricSubspaceKey {
		GaugeMetricSubspaceKey::new()
	}
}

impl FormalKey for GaugeMetricKey {
	// IMPORTANT: Uses LE bytes, not BE
	/// Count.
	type Value = usize;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(usize::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for GaugeMetricKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (METRIC, self.metric.variant() as usize);
		offset += t.pack(w, tuple_depth)?;

		offset += match &self.metric {
			GaugeMetric::WorkflowActive(workflow_name) => workflow_name.pack(w, tuple_depth)?,
			GaugeMetric::WorkflowSleeping(workflow_name) => workflow_name.pack(w, tuple_depth)?,
			GaugeMetric::WorkflowDead(workflow_name, error) => {
				(workflow_name, error).pack(w, tuple_depth)?
			}
			GaugeMetric::WorkflowComplete(workflow_name) => workflow_name.pack(w, tuple_depth)?,
			GaugeMetric::SignalPending(signal_name) => signal_name.pack(w, tuple_depth)?,
		};

		std::result::Result::Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for GaugeMetricKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, variant)) = <(usize, usize)>::unpack(input, tuple_depth)?;
		let variant = GaugeMetricVariant::from_repr(variant).ok_or_else(|| {
			PackError::Message(format!("invalid metric variant `{variant}` in key").into())
		})?;

		let (input, v) = match variant {
			GaugeMetricVariant::WorkflowActive => {
				let (input, workflow_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					GaugeMetricKey {
						metric: GaugeMetric::WorkflowActive(workflow_name),
					},
				)
			}
			GaugeMetricVariant::WorkflowSleeping => {
				let (input, workflow_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					GaugeMetricKey {
						metric: GaugeMetric::WorkflowSleeping(workflow_name),
					},
				)
			}
			GaugeMetricVariant::WorkflowDead => {
				let (input, (workflow_name, error)) =
					<(String, String)>::unpack(input, tuple_depth)?;

				(
					input,
					GaugeMetricKey {
						metric: GaugeMetric::WorkflowDead(workflow_name, error),
					},
				)
			}
			GaugeMetricVariant::WorkflowComplete => {
				let (input, workflow_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					GaugeMetricKey {
						metric: GaugeMetric::WorkflowComplete(workflow_name),
					},
				)
			}
			GaugeMetricVariant::SignalPending => {
				let (input, signal_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					GaugeMetricKey {
						metric: GaugeMetric::SignalPending(signal_name),
					},
				)
			}
		};

		std::result::Result::Ok((input, v))
	}
}

pub struct GaugeMetricSubspaceKey {}

impl GaugeMetricSubspaceKey {
	pub fn new() -> Self {
		GaugeMetricSubspaceKey {}
	}
}

impl TuplePack for GaugeMetricSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (METRIC,);
		t.pack(w, tuple_depth)
	}
}
