use uuid::Uuid;

const SENTRY_URL: &str = "https://7602663e43cb9dee8c42d1e5e70293f8@o4504307129188352.ingest.us.sentry.io/4509962797252608";

// We use synchronous main for Sentry. Read more: https://docs.sentry.io/platforms/rust/#async-main-function
pub fn init(config: &rivet_config::Config) -> Option<sentry::ClientInitGuard> {
	if !config.telemetry.enabled {
		return None;
	}

	let guard = sentry::init((
		SENTRY_URL,
		sentry::ClientOptions {
			release: sentry::release_name!(),
			..Default::default()
		},
	));

	sentry::configure_scope(|scope| {
		if let Ok(db) = serde_json::to_string(config.database()) {
			scope.set_tag("database", db);
		}
		if let Ok(ps) = serde_json::to_string(config.pubsub()) {
			scope.set_tag("pubsub", ps);
		}
		if let Ok(cache) = serde_json::to_string(config.cache()) {
			scope.set_tag("cache", cache);
		}
		if let Ok(topo) = serde_json::to_string(config.topology()) {
			scope.set_tag("topology", topo);
		}
	});

	Some(guard)
}

pub fn capture_error(err: &anyhow::Error) -> Uuid {
	sentry::integrations::anyhow::capture_anyhow(err)
}
