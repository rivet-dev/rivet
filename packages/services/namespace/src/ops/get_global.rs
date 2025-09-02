use gas::prelude::*;

use crate::types::Namespace;

#[derive(Debug)]
pub struct Input {
	// TODO: Accept vec
	pub namespace_id: Id,
}

#[operation]
pub async fn namespace_get_global(ctx: &OperationCtx, input: &Input) -> Result<Option<Namespace>> {
	if ctx.config().is_leader() {
		let namespaces_res = ctx
			.op(crate::ops::get_local::Input {
				namespace_ids: vec![input.namespace_id],
			})
			.await?;

		Ok(namespaces_res.namespaces.into_iter().next())
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		let client = rivet_pools::reqwest::client().await?;

		ctx.cache()
			.clone()
			.request()
			.fetch_one_json("namespace.get_global", input.namespace_id, {
				let leader_dc = leader_dc.clone();
				let client = client.clone();
				move |mut cache, key| {
					let leader_dc = leader_dc.clone();
					let client = client.clone();
					async move {
						let url = leader_dc
							.api_peer_url
							.join(&format!("/namespaces/{}", input.namespace_id))?;
						let res = client.get(url).send().await?;

						let res = rivet_api_util::parse_response::<GetResponse>(res).await;

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
struct GetResponse {
	namespace: Namespace,
}
