use prometheus::*;

lazy_static::lazy_static! {
	pub static ref REGISTRY: Registry = Registry::new_custom(
		Some("pegboard".to_string()),
		Some(labels! {
			"client_id".to_owned() => std::env::var("CLIENT_ID").unwrap_or_default(),
		}),
	).unwrap();
}
