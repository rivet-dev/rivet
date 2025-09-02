use gas::prelude::*;

/// Request identifiers that are passed through the request lifecycle
#[derive(Debug, Clone, Copy)]
pub struct RequestIds {
	pub ray_id: Id,
	pub req_id: Id,
}

impl RequestIds {
	pub fn new(dc_label: u16) -> Self {
		Self {
			ray_id: Id::new_v1(dc_label),
			req_id: Id::new_v1(dc_label),
		}
	}
}
