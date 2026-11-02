//! Scope-bound effect clients selected above Env. These checks stop fresh dispatch after coordinator
//! loss; domain conditional writes and monotonic deliveries still fence already-submitted mutations.
use anyhow::{Result, ensure};
use bytes::Bytes;
use rivet_env::capabilities::http::{HttpBody, HttpClient, HttpFuture, Request, Response};
use rivet_singleton_coordinator::authority::Authority;
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

pub(crate) struct SingletonScope {
	pub authority: Authority,
	alive: Arc<AtomicBool>,
	pub depot: depot_client::Client,
	pub http: Arc<dyn HttpClient>,
}
impl SingletonScope {
	pub fn new(
		authority: Authority,
		depot: depot_client::Client,
		http: Arc<dyn HttpClient>,
	) -> Result<Self> {
		let alive = Arc::new(AtomicBool::new(true));
		let gate = authority.clone();
		let gate_alive = alive.clone();
		let depot = depot.with_dispatch_guard(Arc::new(move || {
			gate_alive.load(Ordering::SeqCst) && gate.permits()
		}));
		let http: Arc<dyn HttpClient> = Arc::new(ScopedHttp {
			alive: alive.clone(),
			authority: authority.clone(),
			inner: http,
		});
		Ok(Self {
			alive,
			authority,
			depot,
			http,
		})
	}
}
impl SingletonScope {
	pub fn close(&self) {
		self.alive.store(false, Ordering::SeqCst);
	}
}
impl Drop for SingletonScope {
	fn drop(&mut self) {
		self.alive.store(false, Ordering::SeqCst);
	}
}
struct ScopedHttp {
	alive: Arc<AtomicBool>,
	authority: Authority,
	inner: Arc<dyn HttpClient>,
}
impl HttpClient for ScopedHttp {
	fn execute(&self, request: Request<Bytes>, limit: usize) -> HttpFuture<'_, Response<Bytes>> {
		Box::pin(async move {
			ensure!(
				self.alive.load(Ordering::SeqCst) && self.authority.permits(),
				"singleton HTTP dispatch was revoked"
			);
			self.inner.execute(request, limit).await
		})
	}
	fn execute_stream(&self, request: Request<HttpBody>) -> HttpFuture<'_, Response<HttpBody>> {
		Box::pin(async move {
			ensure!(
				self.alive.load(Ordering::SeqCst) && self.authority.permits(),
				"singleton HTTP dispatch was revoked"
			);
			self.inner.execute_stream(request).await
		})
	}
}
