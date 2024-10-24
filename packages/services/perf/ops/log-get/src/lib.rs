use futures_util::stream::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "perf-log-get")]
async fn handle(
	ctx: OperationContext<perf::log_get::Request>,
) -> GlobalResult<perf::log_get::Response> {
	let ray_ids = ctx
		.ray_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch all rays
	let redis_pool = ctx.redis_cache().await?;
	let rays = futures_util::stream::iter(ray_ids)
		.map(move |ray_id| {
			let mut redis_pool = redis_pool.clone();
			async move {
				let key = format!("chirp-perf:ray:{}", ray_id);
				let mut pipe = redis::pipe();

				// Fetch all requests in a given ray
				pipe.hvals(&key);
				let (res,) = pipe
					.query_async::<_, (Vec<Vec<u8>>,)>(&mut redis_pool)
					.await?;
				pipe.clear();

				tracing::info!(?ray_id, bytes = %res.len(), "requests response");

				GlobalResult::Ok((ray_id, res))
			}
		})
		.buffer_unordered(16)
		.map(|res| {
			match res {
				Ok((ray_id, res)) => {
					// Decode all perf spans
					let perf_lists = res
						.into_iter()
						.map(|perf_item| Ok(proto::perf::SvcPerf::decode(perf_item.as_slice())?))
						.collect::<GlobalResult<Vec<_>>>()?;

					// Collect spans
					GlobalResult::Ok(perf::log_get::response::Ray {
						ray_id: Some(ray_id.into()),
						perf_lists,
					})
				}
				Err(err) => GlobalResult::Err(err),
			}
		})
		.try_collect::<Vec<_>>()
		.await?;

	Ok(perf::log_get::Response { rays })
}
