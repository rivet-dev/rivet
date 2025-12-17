use prometheus::*;

lazy_static::lazy_static! {
	pub static ref REGISTRY: Registry = Registry::new_custom(
		Some("rivet".to_string()),
		Some(labels! {
			"service".to_owned() => rivet_env::service_name().to_string(),
			"kubernetes_pod_id".to_owned() => std::env::var("KUBERNETES_POD_ID").unwrap_or_default(),
			"worker_source_hash".to_owned() => rivet_env::source_hash().to_string(),
		})).unwrap();
}
