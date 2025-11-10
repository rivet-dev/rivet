use gas::prelude::*;
use rivet_types::namespaces::Namespace;

#[derive(Debug)]
pub struct Input {
	pub namespace_ids: Vec<Id>,
}

#[operation]
pub async fn namespace_get_global(ctx: &OperationCtx, input: &Input) -> Result<Vec<Namespace>> {
	if ctx.config().is_leader() {
		ctx.op(super::get_local::Input {
			namespace_ids: input.namespace_ids.clone(),
		})
		.await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		let client = rivet_pools::reqwest::client().await?;

		ctx.cache()
			.clone()
			.request()
			.fetch_all_json("namespace.get_global", input.namespace_ids.clone(), {
				let leader_dc = leader_dc.clone();
				let client = client.clone();
				move |mut cache, namespace_ids| {
					let leader_dc = leader_dc.clone();
					let client = client.clone();
					async move {
						let url = leader_dc.peer_url.join(&format!("/namespaces"))?;
						let res = client
							.get(url)
							.query(&rivet_api_types::namespaces::list::ListQuery {
								namespace_ids: Some(
									namespace_ids
										.iter()
										.map(|ns_id| ns_id.to_string())
										.collect::<Vec<_>>()
										.join(","),
								),
								limit: None,
								cursor: None,
								name: None,
							})
							.send()
							.await?;

						let res = rivet_api_util::parse_response::<
							rivet_api_types::namespaces::list::ListResponse,
						>(res)
						.await?;

						for ns in res.namespaces {
							let namespace_id = ns.namespace_id;
							cache.resolve(&&namespace_id, ns);
						}

						Ok(cache)
					}
				}
			})
			.await
	}
}
