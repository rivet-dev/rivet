use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

fn convert_datacenter(
	datacenter: &backend::cluster::Datacenter,
	locations: &[cluster::datacenter_location_get::response::Datacenter],
) -> GlobalResult<backend::region::Region> {
	let datacenter_id = unwrap_ref!(datacenter.datacenter_id).as_uuid();
	let provider = unwrap!(backend::cluster::Provider::from_i32(datacenter.provider));

	let coords = locations
		.iter()
		.find(|location| location.datacenter_id == datacenter.datacenter_id)
		.and_then(|dc| dc.coords.clone())
		.unwrap_or(backend::net::Coordinates {
			latitude: 0.0,
			longitude: 0.0,
		});

	Ok(backend::region::Region {
		region_id: datacenter.datacenter_id,
		enabled: true,
		nomad_region: "global".into(),
		nomad_datacenter: datacenter_id.to_string(),
		provider: match provider {
			backend::cluster::Provider::Linode => "linode".to_string(),
		},
		provider_region: datacenter.provider_datacenter_id.clone(),
		provider_display_name: match provider {
			backend::cluster::Provider::Linode => "Linode".to_string(),
		},
		region_display_name: datacenter.display_name.clone(),
		name_id: datacenter.name_id.clone(),
		coords: Some(coords),

		build_delivery_method: datacenter.build_delivery_method,
	})
}

#[operation(name = "region-get")]
async fn handle(
	ctx: OperationContext<region::get::Request>,
) -> GlobalResult<region::get::Response> {
	let (datacenters_res, locations_res) = tokio::try_join!(
		op!([ctx] cluster_datacenter_get {
			datacenter_ids: ctx.region_ids.clone(),
		}),
		op!([ctx] cluster_datacenter_location_get {
			datacenter_ids: ctx.region_ids.clone(),
		}),
	)?;

	let regions = datacenters_res
		.datacenters
		.iter()
		.map(|dc| convert_datacenter(dc, &locations_res.datacenters))
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(region::get::Response { regions })
}
