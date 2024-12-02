use anyhow::*;
use serde::{Deserialize, Serialize};

use crate::{meta, paths, util::task};

#[derive(Deserialize)]
pub struct Input {}

#[derive(Serialize)]
pub struct Output {}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"auth.sign_out"
	}

	async fn run(_task: task::TaskCtx, _input: Self::Input) -> Result<Self::Output> {
		meta::mutate_project(&paths::data_dir()?, |meta| {
			meta.cloud = None;
		})
		.await?;
		Ok(Output {})
	}
}
