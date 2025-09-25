use gas::prelude::*;
use rivet_types::namespaces::Namespace;

#[derive(Debug)]
pub struct Input {
	pub name: String,
}

#[operation]
pub async fn namespace_resolve_for_name_global(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<Namespace>> {
	if ctx.config().is_leader() {
		ctx.op(crate::ops::resolve_for_name_local::Input {
			name: input.name.clone(),
		})
		.await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		let client = rivet_pools::reqwest::client().await?;

		ctx.cache()
			.clone()
			.request()
			.fetch_one_json("namespace.resolve_for_name_global", input.name.clone(), {
				let leader_dc = leader_dc.clone();
				let client = client.clone();
				move |mut cache, key| {
					let leader_dc = leader_dc.clone();
					let client = client.clone();
					async move {
						let url = leader_dc.api_peer_url.join(&format!("/namespaces"))?;
						let res = client.get(url).query(&("name", &input.name)).send().await?;

						let res = rivet_api_util::parse_response::<
							rivet_api_types::namespaces::list::ListResponse,
						>(res)
						.await?;

						let ns = res.namespaces.into_iter().next();

						cache.resolve(&key, ns);

						Ok(cache)
					}
				}
			})
			.await
			.map(|x| x.flatten())
	}
}
