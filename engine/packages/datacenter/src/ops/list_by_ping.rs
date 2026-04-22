use futures_util::StreamExt;
use gas::prelude::*;
use universaldb::prelude::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {}

#[derive(Debug)]
pub struct Output {
	/// In ascending order by rtt.
	pub datacenters: Vec<Datacenter>,
}

#[derive(Debug)]
pub struct Datacenter {
	pub dc_label: u16,
	pub rtt: u32,
}

#[operation]
pub async fn datacenter_list_by_ping(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_labels = ctx
		.config()
		.topology()
		.datacenters
		.iter()
		// Exclude current dc
		.filter(|dc| dc.datacenter_label != ctx.config().dc_label())
		.map(|dc| dc.datacenter_label)
		.collect::<Vec<_>>();

	let rtt = ctx
		.cache()
		.clone()
		.request()
		.ttl(crate::workflows::ping::TICK_RATE.as_millis() as i64)
		.fetch_all_json(
			"datacenter.ping",
			dc_labels.clone(),
			move |mut cache, dc_labels| async move {
				let res = ctx
					.udb()?
					.run(|tx| {
						let dc_labels = dc_labels.clone();

						async move {
							let tx = tx.with_subspace(keys::subspace());

							let res = futures_util::stream::iter(dc_labels)
								.map(|dc_label| {
									let tx = tx.clone();

									async move {
										(
											dc_label,
											tx.read(&keys::LastRttKey::new(dc_label), Serializable)
												.await
												.unwrap_or(0),
										)
									}
								})
								.buffer_unordered(128)
								.collect::<Vec<_>>()
								.await;

							Ok(res)
						}
					})
					.await?;

				for (dc_label, rtt) in res {
					cache.resolve(&dc_label, rtt);
				}

				Ok(cache)
			},
		)
		.await?;

	// Add current dc
	let mut datacenters = std::iter::once(Datacenter {
		dc_label: ctx.config().dc_label(),
		rtt: 0,
	})
	.chain(
		dc_labels
			.into_iter()
			.zip(rtt)
			.map(|(dc_label, rtt)| Datacenter { dc_label, rtt }),
	)
	.collect::<Vec<_>>();

	datacenters.sort_by_key(|dc| dc.rtt);

	Ok(Output { datacenters })
}
