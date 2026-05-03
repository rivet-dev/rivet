use anyhow::Result;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

use crate::custom_serve::CustomServeTrait;
use crate::metrics;
use crate::request_context::RequestContext;

// Routing types
#[derive(Clone, Debug)]
pub struct RouteTarget {
	pub host: String,
	pub port: u16,
	pub path: String,
}

#[derive(Clone, Debug)]
pub struct RouteConfig {
	pub targets: Vec<RouteTarget>,
}

#[derive(Clone)]
pub enum RoutingOutput {
	/// Return the data to route to.
	Route(RouteConfig),
	/// Return a custom serve handler.
	CustomServe(Arc<dyn CustomServeTrait>),
}

#[derive(Clone)]
pub(crate) enum ResolveRouteOutput {
	Target(RouteTarget),
	CustomServe(Arc<dyn CustomServeTrait>),
}

pub type RoutingFn = Arc<
	dyn for<'a> Fn(&'a mut RequestContext) -> futures::future::BoxFuture<'a, Result<RoutingOutput>>
		+ Send
		+ Sync,
>;

pub type CacheKeyFn = Arc<dyn for<'a> Fn(&'a mut RequestContext) -> Result<u64> + Send + Sync>;

// Cache for routing results
pub(crate) struct RouteCache {
	cache: Cache<u64, RoutingOutput>,
}

impl RouteCache {
	pub(crate) fn new(ttl: Duration) -> Self {
		Self {
			cache: Cache::builder()
				.max_capacity(10_000)
				.time_to_live(ttl)
				.build(),
		}
	}

	#[tracing::instrument(skip_all)]
	pub(crate) async fn get(&self, key: &u64) -> Option<RoutingOutput> {
		self.cache.get(key).await
	}

	// TODO: Disable route caching for now, determine edge cases with gateway
	#[allow(dead_code)]
	#[tracing::instrument(skip_all)]
	pub(crate) async fn insert(&self, key: u64, output: RoutingOutput) {
		self.cache.insert(key, output).await;

		metrics::ROUTE_CACHE_COUNT.set(self.cache.entry_count() as i64);
	}
}
