use std::fmt;
use std::future::Future;
use std::io::Cursor;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result};
use rivetkit_client::{Client, ClientConfig, EncodingKind, TransportKind};
use rivetkit_core::actor::schedule::{
	CronFire, CronJobInfo as CoreCronJobInfo, ScheduleKind, ScheduledEventInfo,
};
use rivetkit_core::{
	ActorContext, ActorKey, ActorKv, ConnHandle, ConnId, KeepAwakeRegion, SqliteDb,
	actor::connection::ConnHandles, error::ActorRuntime,
};
use serde::{Serialize, de::DeserializeOwned};
use tokio_util::sync::CancellationToken;

use crate::action;
use crate::actor::Actor;
use crate::event::Event;

pub struct Ctx<A: Actor> {
	inner: ActorContext,
	client: Arc<OnceLock<Client>>,
	conn: Option<ConnCtx<A>>,
	request: Option<Arc<rivetkit_core::Request>>,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> fmt::Debug for Ctx<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Ctx")
			.field("inner", &self.inner)
			.field("conn_id", &self.conn.as_ref().map(|conn| conn.id()))
			.finish_non_exhaustive()
	}
}

pub struct Schedule<'a> {
	inner: &'a ActorContext,
}

pub struct Cron<'a> {
	inner: &'a ActorContext,
}

pub struct CronSetOptions<'a> {
	pub name: &'a str,
	pub expression: &'a str,
	pub timezone: Option<&'a str>,
	pub action: &'a str,
	pub args: &'a [u8],
	pub max_history: Option<i64>,
}

pub struct CronEveryOptions<'a> {
	pub name: &'a str,
	pub interval: Duration,
	pub action: &'a str,
	pub args: &'a [u8],
	pub max_history: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CronJobInfo {
	pub name: String,
	pub kind: ScheduleKind,
	pub action: String,
	pub args: Vec<u8>,
	pub next_run_at: i64,
	pub last_run_at: Option<i64>,
	pub expression: Option<String>,
	pub timezone: Option<String>,
	pub interval: Option<Duration>,
	pub max_history: i64,
}

impl<A: Actor> Clone for Ctx<A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			conn: self.conn.clone(),
			request: self.request.clone(),
			_p: PhantomData,
		}
	}
}

impl<A: Actor> Ctx<A> {
	pub fn new(inner: ActorContext) -> Self {
		Self {
			inner,
			client: Arc::new(OnceLock::new()),
			conn: None,
			request: None,
			_p: PhantomData,
		}
	}

	pub(crate) fn with_conn(&self, conn: Option<ConnCtx<A>>) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			conn,
			request: self.request.clone(),
			_p: PhantomData,
		}
	}

	pub fn conn(&self) -> Option<&ConnCtx<A>> {
		self.conn.as_ref()
	}

	pub fn request(&self) -> Option<&rivetkit_core::Request> {
		self.request.as_deref()
	}

	pub(crate) fn with_request(&self, request: Option<rivetkit_core::Request>) -> Self {
		let mut ctx = self.clone();
		ctx.request = request.map(Arc::new);
		ctx
	}

	/// Decodes the actor's create input, falling back to the input type's
	/// default when the actor was created without one.
	pub fn input(&self) -> Result<A::Input> {
		match self.inner.input() {
			Some(bytes) => decode_cbor(&bytes, "actor input"),
			None => Ok(A::Input::default()),
		}
	}

	pub fn actor_id(&self) -> &str {
		self.inner.actor_id()
	}

	pub fn name(&self) -> &str {
		self.inner.name()
	}

	pub fn key(&self) -> &ActorKey {
		self.inner.key()
	}

	/// The individual actor forks behind this actor, oldest first; empty when it was never
	/// forked.
	pub fn fork_chain(&self) -> Vec<rivetkit_core::ForkLink> {
		self.inner.fork_chain()
	}

	pub fn region(&self) -> &str {
		self.inner.region()
	}

	/// The name of the namespace the actor runs in. After a namespace fork it differs from the
	/// source actor's namespace while the actor id stays the same.
	pub fn namespace(&self) -> &str {
		self.inner.namespace()
	}

	#[deprecated(
		note = "actor KV was deprecated in 3.0 and will be removed in a future version. Store data in SQLite (`sql()`). See https://rivet.dev/actors/docs/deprecated/kv"
	)]
	#[allow(deprecated)]
	pub fn kv(&self) -> &ActorKv {
		self.inner.kv()
	}

	pub fn sql(&self) -> &SqliteDb {
		self.inner.sql()
	}

	pub fn schedule(&self) -> Schedule<'_> {
		Schedule { inner: &self.inner }
	}

	pub fn cron(&self) -> Cron<'_> {
		Cron { inner: &self.inner }
	}

	/// Holds the actor awake for the duration of `future` and returns its
	/// output. Mirrors the TypeScript `ctx.waitUntil`/keep-awake semantics:
	/// the actor will not sleep while the future is in flight.
	pub async fn keep_awake<F>(&self, future: F) -> F::Output
	where
		F: Future,
	{
		self.inner.keep_awake(future).await
	}

	/// Acquires a keep-awake region that holds the actor awake until the
	/// returned guard is dropped. Use when the awake window does not map
	/// cleanly to a single future.
	pub fn keep_awake_region(&self) -> KeepAwakeRegion {
		self.inner.keep_awake_region()
	}

	/// Registers runtime-owned background work that must drain during shutdown.
	/// Unlike `wait_until`, registered tasks are raced against the shutdown
	/// grace deadline by core.
	pub fn register_task(&self, future: impl Future<Output = ()> + Send + 'static) {
		self.inner.register_task(future);
	}

	/// Returns the actor abort signal. The token is cancelled when the actor is
	/// being torn down so in-flight work can observe shutdown.
	pub fn abort_signal(&self) -> CancellationToken {
		self.inner.actor_abort_signal()
	}

	/// Returns `true` once the actor abort signal has fired.
	pub fn aborted(&self) -> bool {
		self.inner.actor_aborted()
	}

	/// Executes a single SQL statement against the actor database, returning the
	/// CBOR-encoded result. Convenience over `sql()` for the CBOR boundary.
	pub async fn db_exec(&self, sql: &str) -> Result<Vec<u8>> {
		self.inner.db_exec(sql).await
	}

	/// Runs a read query with optional CBOR-encoded bind params, returning
	/// CBOR-encoded rows.
	pub async fn db_query(&self, sql: &str, params: Option<&[u8]>) -> Result<Vec<u8>> {
		self.inner.db_query(sql, params).await
	}

	/// Runs a write query with optional CBOR-encoded bind params, returning the
	/// CBOR-encoded execution result.
	pub async fn db_execute(&self, sql: &str, params: Option<&[u8]>) -> Result<Vec<u8>> {
		self.inner.db_execute(sql, params).await
	}

	/// Runs a statement with optional CBOR-encoded bind params, discarding the
	/// result.
	pub async fn db_run(&self, sql: &str, params: Option<&[u8]>) -> Result<()> {
		self.inner.db_run(sql, params).await
	}

	pub fn sleep(&self) -> Result<()> {
		self.inner.sleep()
	}

	pub fn destroy(&self) -> Result<()> {
		self.inner.destroy()
	}

	/// Requests a stop with an error attached. Behaves like `destroy` locally,
	/// but the engine records the message as a crash (`StopCode::Error`) and
	/// applies its crash handling instead of unconditionally destroying the
	/// actor. Returning an `Err` from `Actor::run` reports the error this way
	/// automatically.
	pub fn stop_with_error(&self, message: impl Into<String>) -> Result<()> {
		self.inner.stop_with_error(message)
	}

	#[deprecated(
		note = "`set_prevent_sleep` was deprecated in 3.0 and will be removed in a future version. It is a no-op. Use `keep_awake` or `wait_until`. See https://rivet.dev/actors/docs/deprecated/prevent-sleep"
	)]
	pub fn set_prevent_sleep(&self, enabled: bool) {
		#[allow(deprecated)]
		self.inner.set_prevent_sleep(enabled);
	}

	#[deprecated(
		note = "`prevent_sleep` was deprecated in 3.0 and will be removed in a future version. It is a no-op that always returns false. Use `keep_awake` or `wait_until`. See https://rivet.dev/actors/docs/deprecated/prevent-sleep"
	)]
	pub fn prevent_sleep(&self) -> bool {
		#[allow(deprecated)]
		self.inner.prevent_sleep()
	}

	pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static) {
		self.inner.wait_until(future);
	}

	pub fn broadcast<E: Serialize>(&self, name: &str, event: &E) -> Result<()> {
		let event_bytes = action::encode_varargs(event, "event args")?;
		self.inner.broadcast(name, &event_bytes);
		Ok(())
	}

	pub fn emit<E: Event>(&self, event: E) -> Result<()> {
		self.broadcast(E::NAME, &event)
	}

	pub fn conns(&self) -> ConnIter<'_, A> {
		ConnIter {
			inner: self.inner.conns(),
			_p: PhantomData,
		}
	}

	pub fn conns_vec(&self) -> Vec<ConnCtx<A>> {
		self.conns().collect()
	}

	pub async fn disconnect_conn(&self, id: &ConnId) -> Result<()> {
		self.inner.disconnect_conn(id.clone()).await
	}

	pub(crate) async fn disconnect_conn_with_reason(
		&self,
		id: &ConnId,
		reason: Option<&str>,
	) -> Result<()> {
		self.inner
			.disconnect_conn_with_reason(id.clone(), reason)
			.await
	}

	pub async fn disconnect_conns<F>(&self, pred: F) -> Result<()>
	where
		F: Fn(&ConnCtx<A>) -> bool,
	{
		self.inner
			.disconnect_conns(|conn| pred(&ConnCtx::new(conn.clone())))
			.await
	}

	pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> Result<()> {
		self.inner.set_alarm(timestamp_ms)
	}

	pub fn client(&self) -> Result<Client> {
		if let Some(client) = self.client.get() {
			return Ok(client.clone());
		}

		let endpoint = self.inner.client_endpoint().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "actor client endpoint".to_owned(),
			}
			.build()
		})?;
		let namespace = self.inner.client_namespace().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "actor client namespace".to_owned(),
			}
			.build()
		})?;
		let pool_name = self.inner.client_pool_name().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "actor client pool name".to_owned(),
			}
			.build()
		})?;
		let client = Client::new(
			ClientConfig::new(endpoint)
				.token_opt(self.inner.client_token().map(ToOwned::to_owned))
				.namespace(namespace)
				.pool_name(pool_name)
				.encoding(EncodingKind::Bare)
				.transport(TransportKind::WebSocket)
				.disable_metadata_lookup(true),
		);

		match self.client.set(client) {
			Ok(()) => self.client.get().cloned().ok_or_else(|| {
				ActorRuntime::NotConfigured {
					component: "actor client cache".to_owned(),
				}
				.build()
			}),
			Err(client) => Ok(self.client.get().cloned().unwrap_or(client)),
		}
	}

	pub fn inner(&self) -> &ActorContext {
		&self.inner
	}

	pub fn into_inner(self) -> ActorContext {
		self.inner
	}
}

impl Schedule<'_> {
	pub async fn after(
		&self,
		duration: std::time::Duration,
		action_name: &str,
		args: &[u8],
	) -> anyhow::Result<String> {
		self.inner.after(duration, action_name, args).await
	}

	pub async fn at(
		&self,
		timestamp_ms: i64,
		action_name: &str,
		args: &[u8],
	) -> anyhow::Result<String> {
		self.inner.at(timestamp_ms, action_name, args).await
	}

	pub async fn cancel(&self, event_id: &str) -> anyhow::Result<bool> {
		self.inner.cancel_schedule(event_id).await
	}

	pub async fn get(&self, event_id: &str) -> anyhow::Result<Option<ScheduledEventInfo>> {
		self.inner.get_scheduled_event(event_id).await
	}

	pub async fn list(&self) -> anyhow::Result<Vec<ScheduledEventInfo>> {
		self.inner.list_scheduled_events().await
	}
}

impl Cron<'_> {
	pub async fn set(&self, options: CronSetOptions<'_>) -> anyhow::Result<()> {
		self.inner
			.cron_set(
				options.name,
				options.expression,
				options.timezone,
				options.action,
				options.args,
				options.max_history,
			)
			.await
	}

	pub async fn every(&self, options: CronEveryOptions<'_>) -> anyhow::Result<()> {
		let interval_ms = i64::try_from(options.interval.as_millis()).unwrap_or(i64::MAX);
		self.inner
			.cron_every(
				options.name,
				interval_ms,
				options.action,
				options.args,
				options.max_history,
			)
			.await
	}

	pub async fn get(&self, name: &str) -> anyhow::Result<Option<CronJobInfo>> {
		self.inner
			.cron_get(name)
			.await?
			.map(cron_job_info_from_core)
			.transpose()
	}

	pub async fn list(&self) -> anyhow::Result<Vec<CronJobInfo>> {
		self.inner
			.cron_list()
			.await?
			.into_iter()
			.map(cron_job_info_from_core)
			.collect()
	}

	pub async fn delete(&self, name: &str) -> anyhow::Result<bool> {
		self.inner.cron_delete(name).await
	}

	pub async fn history(&self, name: &str, limit: Option<i64>) -> anyhow::Result<Vec<CronFire>> {
		self.inner.cron_history(name, limit).await
	}
}

fn cron_job_info_from_core(value: CoreCronJobInfo) -> anyhow::Result<CronJobInfo> {
	let interval = value
		.interval_ms
		.map(|interval| {
			u64::try_from(interval)
				.context("schedule interval is negative")
				.map(Duration::from_millis)
		})
		.transpose()?;
	Ok(CronJobInfo {
		name: value.name,
		kind: value.kind,
		action: value.action,
		args: value.args,
		next_run_at: value.next_run_at,
		last_run_at: value.last_run_at,
		expression: value.expression,
		timezone: value.timezone,
		interval,
		max_history: value.max_history,
	})
}

pub struct ConnIter<'a, A: Actor> {
	inner: ConnHandles<'a>,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> ConnIter<'_, A> {
	pub fn len(&self) -> usize {
		self.inner.len()
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

impl<A: Actor> Iterator for ConnIter<'_, A> {
	type Item = ConnCtx<A>;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(ConnCtx::new)
	}
}

#[derive(Debug)]
pub struct ConnCtx<A: Actor> {
	inner: ConnHandle,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> Clone for ConnCtx<A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_p: PhantomData,
		}
	}
}

impl<A: Actor> ConnCtx<A> {
	pub(crate) fn new(inner: ConnHandle) -> Self {
		Self {
			inner,
			_p: PhantomData,
		}
	}

	pub fn id(&self) -> &str {
		self.inner.id()
	}

	pub fn is_hibernatable(&self) -> bool {
		self.inner.is_hibernatable()
	}

	pub fn params(&self) -> Result<A::ConnParams> {
		decode_cbor(&self.inner.params(), "connection params")
	}

	pub fn send<E: Serialize>(&self, name: &str, event: &E) -> Result<()> {
		let event_bytes = action::encode_varargs(event, "connection event args")?;
		self.inner.send(name, &event_bytes);
		Ok(())
	}

	pub async fn disconnect(&self, reason: Option<&str>) -> Result<()> {
		self.inner.disconnect(reason).await
	}

	pub fn inner(&self) -> &ConnHandle {
		&self.inner
	}

	pub fn into_inner(self) -> ConnHandle {
		self.inner
	}
}

impl<A: Actor> From<ConnHandle> for ConnCtx<A> {
	fn from(value: ConnHandle) -> Self {
		Self::new(value)
	}
}

fn decode_cbor<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
	ciborium::from_reader(Cursor::new(bytes)).with_context(|| format!("decode {label} from cbor"))
}
