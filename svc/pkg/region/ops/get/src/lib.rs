use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

fn convert_datacenter(
	datacenter: &cluster::types::Datacenter,
	locations: &[cluster::ops::datacenter::location_get::Datacenter],
) -> GlobalResult<backend::region::Region> {
	let coords = locations
		.iter()
		.find(|location| location.datacenter_id == datacenter.datacenter_id)
		.map(|dc| dc.coords.clone())
		.unwrap_or(cluster::ops::datacenter::location_get::Coordinates {
			latitude: 0.0,
			longitude: 0.0,
		});

	Ok(backend::region::Region {
		region_id: Some(datacenter.datacenter_id.into()),
		enabled: true,
		nomad_region: "global".into(),
		nomad_datacenter: datacenter.datacenter_id.to_string(),
		provider: match datacenter.provider {
			cluster::types::Provider::Linode => "linode".to_string(),
		},
		provider_region: datacenter.provider_datacenter_id.clone(),
		provider_display_name: match datacenter.provider {
			cluster::types::Provider::Linode => "Linode".to_string(),
		},
		region_display_name: datacenter.display_name.clone(),
		name_id: datacenter.name_id.clone(),
		coords: Some(backend::net::Coordinates {
			latitude: coords.latitude,
			longitude: coords.longitude,
		}),

		build_delivery_method: match datacenter.build_delivery_method {
			cluster::types::BuildDeliveryMethod::TrafficServer => {
				backend::cluster::BuildDeliveryMethod::TrafficServer as i32
			}
			cluster::types::BuildDeliveryMethod::S3Direct => {
				backend::cluster::BuildDeliveryMethod::S3Direct as i32
			}
		},
	})
}

#[operation(name = "region-get")]
async fn handle(
	ctx: OperationContext<region::get::Request>,
) -> GlobalResult<region::get::Response> {
	let datacenter_ids = ctx
		.region_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();
	let (datacenters_res, locations_res) = tokio::try_join!(
		chirp_workflow::compat::op(
			&ctx,
			cluster::ops::datacenter::get::Input {
				datacenter_ids: datacenter_ids.clone(),
			},
		),
		chirp_workflow::compat::op(
			&ctx,
			cluster::ops::datacenter::location_get::Input {
				datacenter_ids: datacenter_ids.clone(),
			},
		),
	)?;

	let regions = datacenters_res
		.datacenters
		.iter()
		.map(|dc| convert_datacenter(dc, &locations_res.datacenters))
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(region::get::Response { regions })
}
