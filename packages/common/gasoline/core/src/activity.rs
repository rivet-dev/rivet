use std::{fmt::Debug, hash::Hash};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};

use crate::ctx::ActivityCtx;

#[async_trait]
pub trait Activity {
	type Input: ActivityInput;
	type Output: Serialize + DeserializeOwned + Debug + Send;

	const NAME: &'static str;
	const MAX_RETRIES: usize;
	const TIMEOUT: std::time::Duration;

	async fn run(ctx: &ActivityCtx, input: &Self::Input) -> Result<Self::Output>;
}

pub trait ActivityInput: Serialize + DeserializeOwned + Debug + Hash + Send {
	type Activity: Activity;
}
