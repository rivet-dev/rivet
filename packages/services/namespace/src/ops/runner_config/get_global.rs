use gas::prelude::*;
use rivet_types::namespaces::RunnerConfig;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Input {
	pub runners: Vec<(Id, String)>,
}

#[operation]
pub async fn namespace_runner_config_get_global(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<super::get_local::RunnerConfig>> {
	if ctx.config().is_leader() {
		ctx.op(super::get_local::Input {
			runners: input.runners.clone(),
		})
		.await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		let client = rivet_pools::reqwest::client().await?;

		ctx.cache()
			.clone()
			.request()
			.fetch_all_json(
				&format!("namespace.runner_config.get_global"),
				input.runners.clone(),
				{
					let leader_dc = leader_dc.clone();
					let client = client.clone();

					move |mut cache, runners| {
						let leader_dc = leader_dc.clone();
						let client = client.clone();

						async move {
							let namespaces = ctx
								.op(crate::ops::get_global::Input {
									namespace_ids: runners
										.iter()
										.map(|(ns_id, _)| *ns_id)
										.collect(),
								})
								.await?;

							let mut runner_names_by_namespace_id =
								HashMap::with_capacity(runners.len());

							for (namespace_id, runner_name) in runners {
								let runner_names = runner_names_by_namespace_id
									.entry(namespace_id)
									.or_insert_with(Vec::new);
								runner_names.push(runner_name);
							}

							// TODO: Parallelize
							for (namespace_id, runner_names) in runner_names_by_namespace_id {
								let namespace = namespaces
									.iter()
									.find(|n| n.namespace_id == namespace_id)
									.context("namespace not found")?;
								let url = leader_dc.api_peer_url.join("/runner-configs")?;
								let res = client
									.get(url)
									.query(&[("namespace", &namespace.name)])
									.query(
										&runner_names
											.iter()
											.map(|runner_name| ("runner", runner_name))
											.collect::<Vec<_>>(),
									)
									.send()
									.await?;

								let res =
									rivet_api_util::parse_response::<RunnerConfigListResponse>(res)
										.await?;

								for (runner_name, runner_config) in res.runner_configs {
									cache.resolve(
										&(namespace_id, runner_name.clone()),
										super::get_local::RunnerConfig {
											namespace_id,
											name: runner_name,
											config: runner_config,
										},
									);
								}
							}

							Ok(cache)
						}
					}
				},
			)
			.await
	}
}

// TODO: Cyclical dependency with api_peer
#[derive(Deserialize)]
struct RunnerConfigListResponse {
	runner_configs: HashMap<String, RunnerConfig>,
}
