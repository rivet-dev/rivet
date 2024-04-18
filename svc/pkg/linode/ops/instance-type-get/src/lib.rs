use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use util_linode::api;

#[operation(name = "linode-instance-type-get")]
pub async fn handle(
	ctx: OperationContext<linode::instance_type_get::Request>,
) -> GlobalResult<linode::instance_type_get::Response> {
	// Build HTTP client
	let client = util_linode::Client::new(None).await?;

	// Get hardware stats from linode and cache
	let instance_types_res = ctx
		.cache()
		.ttl(util::duration::days(1))
		.fetch_one_proto("instance_types", "linode", {
			let client = client.clone();
			move |mut cache, key| {
				let client = client.clone();
				async move {
					let api_res = api::list_instance_types(&client).await?;

					cache.resolve(
						&key,
						linode::instance_type_get::CacheInstanceTypes {
							instance_types: api_res.into_iter().map(Into::into).collect::<Vec<_>>(),
						},
					);

					Ok(cache)
				}
			}
		})
		.await?;

	let instance_types = unwrap!(instance_types_res)
		.instance_types
		.into_iter()
		.filter(|ty| ctx.hardware_ids.iter().any(|h| h == &ty.hardware_id))
		.collect::<Vec<_>>();

	Ok(linode::instance_type_get::Response { instance_types })
}
