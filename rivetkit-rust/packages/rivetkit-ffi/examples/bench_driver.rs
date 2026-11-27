//! Client side of the SDK comparison benchmark. Drives a `counter` actor served by
//! any SDK (Rust, Java, TypeScript) through the same Rust client, so only the
//! server-side SDK differs between runs.
//!
//! Usage: bench_driver <label> <pool> [seq_calls] [actors] [calls_per_actor]
//! Env: RIVET_ENDPOINT (default http://127.0.0.1:6420).
//!
//! Prints one `RESULT` line per scenario:
//! - `seq`: one actor, sequential calls (latency).
//! - `par`: many actors, each called sequentially, all at once (throughput).

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rivetkit_client::{Client, ClientConfig, GetOrCreateOptions};
use serde_json::json;

fn arg(args: &[String], i: usize, default: usize) -> usize {
	args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn percentile(sorted: &[Duration], p: f64) -> f64 {
	let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
	sorted[i].as_secs_f64() * 1e6
}

async fn call_with_retry(handle: &rivetkit_client::handle::ActorHandle) -> Result<()> {
	let deadline = Instant::now() + Duration::from_secs(60);
	loop {
		match handle.action("increment", vec![json!(1)]).await {
			Ok(_) => return Ok(()),
			Err(error) if Instant::now() < deadline => {
				let msg = error.to_string();
				// Setup only: a retried create may apply one extra increment, which
				// does not matter here.
				if msg.contains("actor_ready_timeout")
					|| msg.contains("request_timeout")
					|| msg.contains("no_runner_config_configured")
					|| msg.contains("Service Unavailable")
				{
					tokio::time::sleep(Duration::from_millis(250)).await;
					continue;
				}
				return Err(error);
			}
			Err(error) => return Err(error),
		}
	}
}

fn report(label: &str, scenario: &str, mut samples: Vec<Duration>, total: Duration) {
	samples.sort();
	let n = samples.len();
	println!(
		"RESULT sdk={label} scenario={scenario} calls={n} total_ms={:.0} calls_per_s={:.0} p50_us={:.0} p90_us={:.0} p99_us={:.0}",
		total.as_secs_f64() * 1e3,
		n as f64 / total.as_secs_f64(),
		percentile(&samples, 0.50),
		percentile(&samples, 0.90),
		percentile(&samples, 0.99),
	);
}

#[tokio::main]
async fn main() -> Result<()> {
	let args: Vec<String> = std::env::args().collect();
	let label = args
		.get(1)
		.context("usage: bench_driver <label> <pool> ...")?
		.clone();
	let pool = args
		.get(2)
		.context("usage: bench_driver <label> <pool> ...")?
		.clone();
	let seq_calls = arg(&args, 3, 10_000);
	let actors = arg(&args, 4, 100);
	let calls_per_actor = arg(&args, 5, 100);
	let endpoint =
		std::env::var("RIVET_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:6420".into());

	let client = Client::new(
		ClientConfig::new(endpoint)
			.token("default")
			.namespace("default")
			.pool_name(pool),
	);
	let run = format!(
		"{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_nanos()
	);

	// Scenario 1: sequential latency on one actor.
	let handle = client.get_or_create(
		"counter",
		vec![format!("seq-{run}")],
		GetOrCreateOptions::default(),
	)?;
	call_with_retry(&handle).await?; // create
	for _ in 0..500 {
		handle.action("increment", vec![json!(1)]).await?; // warm up
	}
	let mut samples = Vec::with_capacity(seq_calls);
	let start = Instant::now();
	for _ in 0..seq_calls {
		let t = Instant::now();
		handle.action("increment", vec![json!(1)]).await?;
		samples.push(t.elapsed());
	}
	report(&label, "seq", samples, start.elapsed());

	// Scenario 2: throughput across many actors.
	let mut handles = Vec::with_capacity(actors);
	for i in 0..actors {
		let h = client.get_or_create(
			"counter",
			vec![format!("par-{run}-{i}")],
			GetOrCreateOptions::default(),
		)?;
		handles.push(h);
	}
	// Create all actors first, 10 at a time, so creation cost is not measured.
	for batch in handles.chunks(10) {
		for c in futures::future::join_all(batch.iter().map(call_with_retry)).await {
			c?;
		}
	}
	let start = Instant::now();
	let tasks: Vec<_> = handles
		.into_iter()
		.map(|h| {
			tokio::spawn(async move {
				let mut samples = Vec::with_capacity(calls_per_actor);
				for _ in 0..calls_per_actor {
					let t = Instant::now();
					h.action("increment", vec![json!(1)]).await?;
					samples.push(t.elapsed());
				}
				Ok::<_, anyhow::Error>(samples)
			})
		})
		.collect();
	let mut all = Vec::with_capacity(actors * calls_per_actor);
	for t in tasks {
		all.extend(t.await??);
	}
	report(
		&label,
		&format!("par{actors}x{calls_per_actor}"),
		all,
		start.elapsed(),
	);
	Ok(())
}
