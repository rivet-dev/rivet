use anyhow::*;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

use super::TaskCtx;

pub trait Task {
	type Input: DeserializeOwned;
	type Output: Serialize;

	fn name() -> &'static str;
	fn run(ctx: TaskCtx, input: Self::Input) -> impl Future<Output = Result<Self::Output>>;
}
