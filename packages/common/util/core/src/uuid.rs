use global_error::prelude::*;
use uuid::Uuid;

pub fn parse(string: &str) -> GlobalResult<Uuid> {
	Uuid::parse_str(string).map_err(|_| err_code!(UUID_INVALID))
}
