use anyhow::*;
use serde::{Deserialize, Serialize};

use crate::{meta, paths, util::task};

#[derive(Deserialize)]
pub struct Input {}

#[derive(Serialize)]
pub struct Output {
	pub signed_in: bool,
}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"auth.check_state"
	}

	async fn run(_task: task::TaskCtx, _input: Input) -> Result<Output> {
		let signed_in =
			meta::read_project(&paths::data_dir()?, |meta| meta.cloud.is_some()).await?;
		Ok(Output { signed_in })
	}
}
