use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use reqwest::{Client, Method};
use rivetkit::{ActorConfig, Request, Response, prelude::*};
use serde_json::{Value, json};

// Persist the generation on sleep; a different value proves we resumed a new instance.
struct ColdStart;

#[async_trait]
impl Actor for ColdStart {
	type State = u64;
	type Input = ();
	type Actions = ();
	type Events = ();
	type Queue = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = action::Raw;

	async fn create_state(_ctx: &Ctx<Self>, _input: ()) -> Result<u64> {
		Ok(0)
	}

	async fn create(ctx: &Ctx<Self>) -> Result<Self> {
		if std::env::var("RIVET_BENCH_READONLY").as_deref() != Ok("1") {
			*ctx.state_mut() += 1;
		}
		Ok(Self)
	}

	async fn on_fetch(self: Arc<Self>, ctx: Ctx<Self>, _req: Request) -> Result<Response> {
		let startup = ctx.sql().startup_report();
		let generation = if std::env::var("RIVET_BENCH_READONLY").as_deref() == Ok("1") {
			startup
				.as_ref()
				.and_then(|s| s["generation"].as_u64())
				.context("missing lease generation")?
		} else {
			*ctx.state()
		};
		Response::from_parts(
			200,
			Default::default(),
			serde_json::to_vec(&json!({"generation":generation,"startup":startup}))?,
		)
	}
}

struct Api {
	client: Client,
	endpoint: String,
	token: String,
	namespace: String,
}

impl Api {
	fn new() -> Result<Self> {
		Ok(Self {
			client: Client::builder().timeout(Duration::from_secs(60)).build()?,
			endpoint: std::env::var("RIVET_ENDPOINT")?,
			token: std::env::var("RIVET_TOKEN")?,
			namespace: std::env::var("RIVET_NAMESPACE")?,
		})
	}

	async fn request(&self, method: Method, path: &str, body: Value) -> Result<Value> {
		let response = self
			.client
			.request(method, format!("{}{path}", self.endpoint))
			.bearer_auth(&self.token)
			.query(&[("namespace", &self.namespace)])
			.json(&body)
			.send()
			.await?;
		let status = response.status();
		let text = response.text().await?;
		ensure!(status.is_success(), "{path}: {status}: {text}");
		Ok(serde_json::from_str(&text)?)
	}

	async fn init(&self) -> Result<()> {
		let response = self
			.client
			.get(format!("{}/namespaces", self.endpoint))
			.bearer_auth(&self.token)
			.query(&[("name", &self.namespace)])
			.send()
			.await?
			.error_for_status()?
			.json::<Value>()
			.await?;
		if response["namespaces"]
			.as_array()
			.context("missing namespaces")?
			.is_empty()
		{
			self.client
				.post(format!("{}/namespaces", self.endpoint))
				.bearer_auth(&self.token)
				.json(&json!({"name": self.namespace, "display_name": "Rust actor benchmarks"}))
				.send()
				.await?
				.error_for_status()?;
		}
		self.request(
			Method::PUT,
			"/runner-configs/default",
			json!({"datacenters": {"default": {"normal": {}}}}),
		)
		.await?;
		Ok(())
	}

	async fn generation(&self, id: &str) -> Result<u64> {
		// Routing headers keep credentials out of the URL. This is a raw HTTP
		// request, so no SDK retry or connection setup is hidden in the sample.
		let response = self
			.client
			.get(format!("{}/request", self.endpoint))
			.header("x-rivet-target", "actor")
			.header("x-rivet-actor", id)
			.header("x-rivet-token", &self.token)
			.header("x-rivet-namespace", &self.namespace)
			.send()
			.await?
			.error_for_status()?;
		let body: Value = response.json().await?;
		if let Some(generation) = body.as_u64() {
			return Ok(generation);
		}
		let startup = &body["startup"];
		ensure!(
			startup["actor_id"].as_str() == Some(id),
			"wrong startup report actor: {startup}"
		);
		if startup["is_new"] == false {
			if std::env::var("RIVET_BENCH_ASSERT_READONLY").as_deref() == Ok("1") {
				ensure!(
					startup["depot_commits"] == 0 && startup["mutating_sql"] == 0,
					"bootstrap wrote storage: {startup}"
				);
			}
			if std::env::var("RIVET_BENCH_ASSERT_PRELOAD").as_deref() == Ok("1") {
				ensure!(
					startup["preload_pages"]
						.as_u64()
						.context("missing preload count")?
						<= startup["page_limit"]
							.as_u64()
							.context("missing preload limit")?,
					"preload exceeded cap: {startup}"
				);
				ensure!(
					startup["assembler_misses"] == 0
						&& startup["vfs_read_misses"] == 0
						&& startup["fallback_page_rpcs"] == 0,
					"preloaded startup missed cache: {startup}"
				);
			}
		}
		eprintln!("startup={startup}");
		body["generation"]
			.as_u64()
			.context("missing response generation")
	}

	async fn sleep(&self, id: &str) -> Result<()> {
		self.request(Method::POST, &format!("/actors/{id}/sleep"), json!({}))
			.await?;
		let deadline = Instant::now() + Duration::from_secs(60);
		loop {
			let response = self
				.request(Method::GET, &format!("/actors?actor_id={id}"), Value::Null)
				.await?;
			let actor = response["actors"]
				.as_array()
				.and_then(|a| a.first())
				.context("actor missing while waiting for sleep")?;
			ensure!(
				actor["destroy_ts"].is_null(),
				"actor was destroyed: {actor}"
			);
			if actor["sleep_ts"].is_number()
				&& actor["connectable_ts"].is_null()
				&& actor["pending_allocation_ts"].is_null()
			{
				return Ok(());
			}
			ensure!(
				Instant::now() < deadline,
				"timed out waiting for sleep: {actor}"
			);
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	}

	async fn measure(&self, id: &str, samples: usize, warmup: usize, cold: bool) -> Result<()> {
		let mut generation = self.generation(id).await.context("initial actor startup")?;
		let mut times = Vec::with_capacity(samples);
		println!(
			"sample,{}",
			if cold {
				"cold_start_ms"
			} else {
				"warm_request_ms"
			}
		);
		for i in 0..samples + warmup {
			if cold {
				self.sleep(id).await?;
				// sleep_ts is published at sleep intent, before shutdown finishes.
				// Allow the tiny actor to finish shutdown outside the timed region.
				tokio::time::sleep(Duration::from_secs(1)).await;
			}
			let start = Instant::now();
			let next = self.generation(id).await?;
			let ms = start.elapsed().as_secs_f64() * 1000.0;
			ensure!(
				next == generation + u64::from(cold),
				"unexpected actor generation (cold={cold}): {generation} -> {next}"
			);
			generation = next;
			if i >= warmup {
				println!("{},{ms:.3}", i - warmup + 1);
				times.push(ms);
			}
		}
		times.sort_by(f64::total_cmp);
		let percentile = |p: f64| times[(times.len() as f64 * p).ceil() as usize - 1];
		eprintln!(
			"n={} mean={:.3}ms p50={:.3}ms p95={:.3}ms p99={:.3}ms min={:.3}ms max={:.3}ms",
			times.len(),
			times.iter().sum::<f64>() / times.len() as f64,
			percentile(0.50),
			percentile(0.95),
			percentile(0.99),
			times[0],
			times[times.len() - 1]
		);
		Ok(())
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let args: Vec<String> = std::env::args().collect();
	match args.get(1).map(String::as_str).unwrap_or("cold-start") {
		"actor" => {
			let mut registry = Registry::new();
			registry.register_actor_with::<ColdStart>(
				"cold-start",
				ActorConfig {
					remote_sqlite: std::env::var("RIVET_BENCH_REMOTE_SQLITE").as_deref() == Ok("1"),
					..Default::default()
				},
			);
			registry.start().await
		}

		"init" => Api::new()?.init().await,
		"cold-start" | "bench" | "warm" => {
			let samples = args
				.get(2)
				.map(|s| s.parse())
				.transpose()?
				.unwrap_or(100usize);
			let warmup = args
				.get(3)
				.map(|s| s.parse())
				.transpose()?
				.unwrap_or(5usize);
			ensure!(
				samples > 0 && samples.checked_add(warmup).is_some(),
				"invalid sample count"
			);
			let api = Api::new()?;
			let created = api
				.request(
					Method::POST,
					"/actors",
					json!({
						"name": "cold-start", "runner_name_selector": "default", "crash_policy": "destroy"
					}),
				)
				.await?;
			let id = created["actor"]["actor_id"]
				.as_str()
				.context("missing actor ID")?;
			eprintln!("actor={id} samples={samples} warmup={warmup}");
			let result = api
				.measure(id, samples, warmup, args.get(1).is_none_or(|s| s != "warm"))
				.await;
			let cleanup = api
				.request(Method::DELETE, &format!("/actors/{id}"), json!({}))
				.await;
			result?;
			cleanup?;
			Ok(())
		}
		other => {
			anyhow::bail!(
				"unknown command {other}; use init, actor, cold-start [samples] [warmup], warm [samples] [warmup]"
			)
		}
	}
}
