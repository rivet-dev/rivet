use std::{fmt::Debug, hash::Hash};

use anyhow::*;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::ActivityCtx;

#[async_trait]
pub trait Activity {
	type Input: ActivityInput;
	type Output: Serialize + DeserializeOwned + Debug + Send;

	fn name() -> &'static str;

	async fn run(ctx: &mut ActivityCtx, input: &Self::Input) -> Result<Self::Output>;
}

pub trait ActivityInput: Serialize + DeserializeOwned + Debug + Hash + Send {
	type Activity: Activity;
}
