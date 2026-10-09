use rivet_config::config::{CacheDriver, Database, PubSub};
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

	sentry::configure_scope(|scope| configure_scope(scope, config));

	Some(guard)
}

fn configure_scope(scope: &mut sentry::Scope, config: &rivet_config::Config) {
	// Only send fixed, allowlisted labels. Config serialization preserves secrets, and
	// even ordinary strings (such as server URLs) can contain credentials.
	scope.set_tag(
		"database",
		match config.database() {
			Database::Postgres(_) => "postgres",
			Database::FileSystem(_) => "file_system",
		},
	);
	scope.set_tag(
		"pubsub",
		match config.pubsub() {
			PubSub::Nats(_) => "nats",
			PubSub::Memory(_) => "memory",
		},
	);
	let cache = config.cache();
	scope.set_tag(
		"cache",
		if cache.enabled {
			match cache.driver() {
				CacheDriver::InMemory => "in_memory",
			}
		} else {
			"disabled"
		},
	);
}

pub fn capture_error(err: &anyhow::Error) -> Uuid {
	sentry::integrations::anyhow::capture_anyhow(err)
}
