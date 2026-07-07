use std::sync::Arc;

use super::*;
use crate::actor::config::ActorConfig;

#[test]
fn actor_metadata_does_not_request_legacy_kv_preload_probes() {
	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), |_start| {
			Box::pin(async { Ok(()) })
		})),
	);

	let metadata = build_actor_metadata_map_from_factories(&factories);
	assert!(
		metadata["counter"].get("preload").is_none(),
		"kv-to-sqlite import is live-scan-only and must not request legacy KV preload probes"
	);
}
