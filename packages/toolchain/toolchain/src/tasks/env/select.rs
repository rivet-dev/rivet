use anyhow::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{meta, paths, util::task};

#[derive(Deserialize)]
pub struct Input {
	pub environment_id: Uuid,
}

#[derive(Serialize)]
pub struct Output {}

#[derive(Serialize)]
pub struct CloudData {}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"env_select"
	}

	async fn run(_task: task::TaskCtx, input: Self::Input) -> Result<Self::Output> {
		meta::try_mutate_project(&paths::data_dir()?, |project| {
			let cloud = project.cloud_mut()?;
			cloud.selected_environment = Some(input.environment_id);
			Ok(())
		})
		.await?;

		Ok(Output {})
	}
}
