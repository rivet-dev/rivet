use anyhow::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use clap::ValueEnum;
use std::time::Duration;
use tokio::signal;
use tokio::sync::watch;
use toolchain::rivet_api::{apis, models};
use uuid::Uuid;

#[derive(ValueEnum, Clone)]
pub enum LogStream {
	#[clap(name = "all")]
	All,
	#[clap(name = "stdout")]
	StdOut,
	#[clap(name = "stderr")]
	StdErr,
}

pub struct TailOpts<'a> {
	pub environment: &'a str,
	pub actor_id: Uuid,
	pub stream: LogStream,
	pub follow: bool,
	pub timestamps: bool,
}

/// Reads the logs of an actor.
pub async fn tail(ctx: &toolchain::ToolchainCtx, opts: TailOpts<'_>) -> Result<()> {
	let (stdout_fetched_tx, stdout_fetched_rx) = watch::channel(false);
	let (stderr_fetched_tx, stderr_fetched_rx) = watch::channel(false);

	tokio::select! {
		result = tail_streams(ctx, &opts, stdout_fetched_tx, stderr_fetched_tx) => result,
		result = poll_actor_state(ctx, &opts, stdout_fetched_rx, stderr_fetched_rx) => result,
		_ = signal::ctrl_c() => {
			Ok(())
		}
	}
}

/// Reads the streams of an actor's logs.
async fn tail_streams(
	ctx: &toolchain::ToolchainCtx,
	opts: &TailOpts<'_>,
	stdout_fetched_tx: watch::Sender<bool>,
	stderr_fetched_tx: watch::Sender<bool>,
) -> Result<()> {
	tokio::try_join!(
		tail_stream(
			ctx,
			&opts,
			models::ActorLogStream::StdOut,
			stdout_fetched_tx
		),
		tail_stream(
			ctx,
			&opts,
			models::ActorLogStream::StdErr,
			stderr_fetched_tx
		),
	)
	.map(|_| ())
}

/// Reads a specific stream of an actor's log.
async fn tail_stream(
	ctx: &toolchain::ToolchainCtx,
	opts: &TailOpts<'_>,
	stream: models::ActorLogStream,
	log_fetched_tx: watch::Sender<bool>,
) -> Result<()> {
	let mut watch_index: Option<String> = None;
	let mut first_batch_fetched = false;

	// Check if this stream is intended to be polled. If not, sleep indefinitely so the other
	// future doesn't exit.
	match (&opts.stream, stream) {
		(LogStream::All, _) => {}
		(LogStream::StdOut, models::ActorLogStream::StdOut) => {}
		(LogStream::StdErr, models::ActorLogStream::StdErr) => {}
		_ => {
			// Notify poll_actor_state
			log_fetched_tx.send(true).ok();

			// Do nothing
			return Ok(());
		}
	}

	loop {
		let res = apis::actor_logs_api::actor_logs_get(
			&ctx.openapi_config_cloud,
			&opts.actor_id.to_string(),
			stream,
			Some(&ctx.project.name_id),
			Some(opts.environment),
			watch_index.as_deref(),
		)
		.await
		.map_err(|err| anyhow!("Failed to fetch logs: {err}"))?;
		watch_index = Some(res.watch.index);

		if !first_batch_fetched {
			log_fetched_tx.send(true).ok();
			first_batch_fetched = true;
		}

		for (ts, line) in res.timestamps.iter().zip(res.lines.iter()) {
			let decoded_line = match STANDARD.decode(line) {
				Result::Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
				Err(_) => {
					eprintln!("Failed to decode base64: {line}");
					continue;
				}
			};

			if opts.timestamps {
				println!("{ts} {decoded_line}");
			} else {
				println!("{decoded_line}");
			}
		}

		if !opts.follow {
			break;
		}
	}

	Ok(())
}

/// Polls the actor state. Exits when finished.
///
/// Using this in a `tokio::select` will make all other tasks cancel when the actor finishes.
async fn poll_actor_state(
	ctx: &toolchain::ToolchainCtx,
	opts: &TailOpts<'_>,
	mut stdout_fetched_rx: watch::Receiver<bool>,
	mut stderr_fetched_rx: watch::Receiver<bool>,
) -> Result<()> {
	// Never resolve if not following this actor in order to just print logs
	if !opts.follow {
		return std::future::pending().await;
	}

	// Wait for the first batch of logs to be fetched before polling actor state.
	//
	// This way, if fetching the logs of an actor, we don't abort the logs until logs have been
	// successfully printed.
	stdout_fetched_rx.changed().await.ok();
	stderr_fetched_rx.changed().await.ok();

	// Poll actor state to shut down when actor finishes
	let mut interval = tokio::time::interval(Duration::from_millis(2_500));
	loop {
		interval.tick().await;

		let res = apis::actor_api::actor_get(
			&ctx.openapi_config_cloud,
			&opts.actor_id.to_string(),
			Some(&ctx.project.name_id),
			Some(opts.environment),
		)
		.await
		.map_err(|err| anyhow!("Failed to poll actor: {err}"))?;

		if res.actor.destroyed_at.is_some() {
			println!("Actor finished");
			return Ok(());
		}
	}
}
