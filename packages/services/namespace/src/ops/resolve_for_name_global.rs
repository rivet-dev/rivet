use gas::prelude::*;

use crate::types::Namespace;

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
						let url = leader_dc
							.api_peer_url
							.join(&format!("/namespaces/resolve/{}", input.name))?;
						let res = client.get(url).send().await?;

						let res =
							rivet_api_util::parse_response::<ResolveForNameResponse>(res).await;

						let res = match res {
							Ok(res) => Ok(Some(res.namespace)),
							Err(err) => {
								// Explicitly handle namespace not found error
								if let Some(error) = err.chain().find_map(|x| {
									x.downcast_ref::<rivet_api_builder::RawErrorResponse>()
								}) {
									if error.1.group == "namespace" && error.1.code == "not_found" {
										Ok(None)
									} else {
										Err(err)
									}
								} else {
									Err(err)
								}
							}
						};

						cache.resolve(&key, res?);

						Ok(cache)
					}
				}
			})
			.await
			.map(|x| x.flatten())
	}
}

// TODO: Cyclical dependency with api_peer
#[derive(Deserialize)]
struct ResolveForNameResponse {
	namespace: Namespace,
}
