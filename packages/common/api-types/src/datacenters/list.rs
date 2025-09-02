use serde::Serialize;
use utoipa::ToSchema;

use crate::pagination::Pagination;

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = DatacentersListResponse)]
pub struct ListResponse {
	pub datacenters: Vec<rivet_types::datacenters::Datacenter>,
	pub pagination: Pagination,
}
