use lazy_static::lazy_static;

lazy_static! {
	pub static ref DISABLE_RATE_LIMIT: bool =
		std::env::var("DEBUG_DISABLE_RATE_LIMIT").map_or(false, |x| x == "1");
}
