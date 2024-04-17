use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

// How much time before the cert expires to renew it
const EXPIRE_PADDING: i64 = util::duration::days(30);

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?
		.wrap_new("cluster-datacenter-tls-renew");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"cluster-datacenter-tls-renew".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	let updated_datacenter_ids = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.base();

		Box::pin(async move {
			// Check for expired rows
			let datacenters = sql_fetch_all!(
				[ctx, (Uuid,), @tx tx]
				"
				SELECT
					datacenter_id
				FROM db_cluster.datacenter_tls
				WHERE
					state = $1 AND
					expire_ts < $2
				FOR UPDATE
				",
				backend::cluster::TlsState::Active as i64,
				util::timestamp::now() + EXPIRE_PADDING,
			)
			.await?
			.into_iter()
			.map(|(datacenter_id,)| datacenter_id)
			.collect::<Vec<_>>();

			// Set as renewing
			for datacenter_id in &datacenters {
				sql_execute!(
					[ctx, @tx tx]
					"
					UPDATE db_cluster.datacenter_tls
					SET state = $2
					WHERE datacenter_id = $1
					",
					datacenter_id,
					backend::cluster::TlsState::Renewing as i64,
				)
				.await?;
			}

			Ok(datacenters)
		})
	})
	.await?;

	for datacenter_id in updated_datacenter_ids {
		msg!([ctx] cluster::msg::datacenter_tls_issue(datacenter_id) {
			datacenter_id: Some(datacenter_id.into()),
			renew: true,
		})
		.await?;
	}

	Ok(())
}
