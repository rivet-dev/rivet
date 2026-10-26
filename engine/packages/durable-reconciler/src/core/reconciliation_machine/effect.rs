use super::types::Slot;
use rivet_state_machine::OpId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect<E> {
	Load(OpId<Slot>),
	Execute(OpId<Slot>, E),
	CancelWork,
	LoadedMetric { success: bool },
}

impl<
	E: Clone + std::fmt::Debug + std::hash::Hash + Eq + Serialize + serde::de::DeserializeOwned + Send,
> rivet_state_machine::Effect for Effect<E>
{
	fn name(&self) -> &'static str {
		match self {
			Effect::Load(..) => "load",
			Effect::Execute(..) => "execute",
			Effect::CancelWork => "cancel_work",
			Effect::LoadedMetric { .. } => "loaded_metric",
		}
	}
}
