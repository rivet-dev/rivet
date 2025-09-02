use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::{datacenters::list::*, pagination::Pagination};
use rivet_types::datacenters::Datacenter;

#[utoipa::path(
    get,
	operation_id = "datacenters_list",
    path = "/datacenters",
    responses(
        (status = 200, body = ListResponse),
    ),
)]
pub async fn list(ctx: ApiCtx, _path: (), _query: ()) -> Result<ListResponse> {
	Ok(ListResponse {
		datacenters: ctx
			.config()
			.topology()
			.datacenters
			.iter()
			.map(|dc| Datacenter {
				datacenter_label: dc.datacenter_label,
				name: dc.name.clone(),
				url: dc.guard_url.to_string(),
			})
			.collect(),
		pagination: Pagination { cursor: None },
	})
}
