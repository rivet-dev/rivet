use std::{fmt::Debug, hash::Hash};

use async_trait::async_trait;
use global_error::GlobalResult;
use serde::{de::DeserializeOwned, Serialize};

use crate::ctx::ActivityCtx;

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
