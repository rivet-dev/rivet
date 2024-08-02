use std::{
	collections::hash_map::DefaultHasher,
	fmt::Debug,
	hash::{Hash, Hasher},
};

use async_trait::async_trait;
use global_error::GlobalResult;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
	ctx::ActivityCtx,
	error::{WorkflowError, WorkflowResult},
};

#[async_trait]
pub trait Activity {
	type Input: ActivityInput;
	type Output: Serialize + DeserializeOwned + Debug + Send;

	const NAME: &'static str;
	const MAX_RETRIES: usize;
	const TIMEOUT: std::time::Duration;

	async fn run(ctx: &ActivityCtx, input: &Self::Input) -> GlobalResult<Self::Output>;
}

pub trait ActivityInput: Serialize + DeserializeOwned + Debug + Hash + Send {
	type Activity: Activity;
}

/// Unique identifier for a specific run of an activity. Used to check for equivalence of activity
/// runs performantly.
///
/// Based on the name of the activity and the hash of the inputs to the activity.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct ActivityId {
	pub name: String,
	pub input_hash: u64,
}

impl ActivityId {
	pub fn new<A: Activity>(input: &A::Input) -> Self {
		let mut hasher = DefaultHasher::new();
		input.hash(&mut hasher);
		let input_hash = hasher.finish();

		Self {
			name: A::NAME.to_string(),
			input_hash,
		}
	}

	pub fn from_bytes(name: String, input_hash: Vec<u8>) -> WorkflowResult<Self> {
		Ok(ActivityId {
			name,
			input_hash: u64::from_le_bytes(
				input_hash
					.try_into()
					.map_err(|_| WorkflowError::IntegerConversion)?,
			),
		})
	}
}
