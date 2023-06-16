use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_cloud_server::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /rays/{}/perf
pub async fn get_ray_perf(
	ctx: Ctx<Auth>,
	ray_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetRayPerfLogsResponse> {
	ctx.auth().admin(ctx.op_ctx()).await?;

	let perf_logs_res = op!([ctx] perf_log_get {
		ray_ids: vec![ray_id.into()],
	})
	.await?;

	let perf_lists = perf_logs_res
		.rays
		.first()
		.map_or(Vec::new(), |ray| ray.perf_lists.clone());

	Ok(models::GetRayPerfLogsResponse {
		perf_lists: perf_lists
			.into_iter()
			.map(ApiTryInto::try_into)
			.collect::<Result<Vec<_>, _>>()?,
	})
}
