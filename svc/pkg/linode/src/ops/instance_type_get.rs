use chirp_workflow::prelude::*;

use crate::{
	types::InstanceType,
	util::{api, client},
};

#[derive(Debug)]
pub struct Input {
	pub hardware_ids: Vec<String>,
}

#[derive(Debug)]
pub struct Output {
	pub instance_types: Vec<InstanceType>,
}

#[operation]
pub async fn linode_instance_type_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	// Build HTTP client
	let client = client::Client::new(None).await?;

	// Get hardware stats from linode and cache
	let instance_types_res = ctx
		.cache()
		.ttl(util::duration::days(1))
		.fetch_one_json("instance_types2", "linode", {
			let client = client.clone();
			move |mut cache, key| {
				let client = client.clone();
				async move {
					let api_res = api::list_instance_types(&client).await?;

					cache.resolve(
						&key,
						api_res
							.into_iter()
							.map(Into::<InstanceType>::into)
							.collect::<Vec<_>>(),
					);

					Ok(cache)
				}
			}
		})
		.await?;

	// Filter by hardware
	let instance_types = unwrap!(instance_types_res)
		.into_iter()
		.filter(|ty| input.hardware_ids.iter().any(|h| h == &ty.hardware_id))
		.collect::<Vec<_>>();

	Ok(Output { instance_types })
}
