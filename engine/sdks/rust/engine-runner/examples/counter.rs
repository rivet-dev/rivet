//! Counter example using the Rust engine runner API.

use anyhow::Result;
use axum::{Json, Router, extract::State, routing::{get, post}};
use rivet_engine_runner::{
	ActorContext, ActorRequestContext, AxumActorDefinition, AxumRunnerApp, Runner, RunnerConfig,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
	let app = AxumRunnerApp::new().with_actor(
		"counter",
		AxumActorDefinition::new(
			Router::new()
				.route("/count", get(get_count))
				.route("/increment", post(increment)),
		)
		.on_start(|ctx: ActorContext| async move {
			tracing::info!(actor_id = %ctx.actor_id, generation = ctx.generation, "counter actor started");
			Ok(())
		})
		.on_stop(|ctx: ActorContext| async move {
			tracing::info!(actor_id = %ctx.actor_id, generation = ctx.generation, "counter actor stopped");
			Ok(())
		}),
	);

	let runner = Runner::builder(
		RunnerConfig::builder()
			.endpoint("http://127.0.0.1:6420")
			.namespace("default")
			.runner_name("counter-runner")
			.build()?,
	)
	.app(app)
	.build()?;

	println!(
		"runner configured. call runner.start().await in an integration environment with a running engine"
	);
	let _ = Arc::new(runner);
	Ok(())
}

async fn get_count(State(ctx): State<ActorRequestContext>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
	let count = ctx
		.kv_get_u64("count")
		.await
		.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
		.unwrap_or(0);
	Ok(Json(json!({ "count": count })))
}

async fn increment(State(ctx): State<ActorRequestContext>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
	let next = ctx
		.kv_get_u64("count")
		.await
		.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
		.unwrap_or(0)
		+ 1;

	ctx.kv_put_u64("count", next)
		.await
		.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

	Ok(Json(json!({ "count": next })))
}
