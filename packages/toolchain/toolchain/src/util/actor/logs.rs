use anyhow::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use clap::ValueEnum;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::signal;
use tokio::sync::watch;
use crate::{
	rivet_api::{apis, models},
	ToolchainCtx
};
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

pub enum PrintType {
	/// Callback that is called when a new line is fetched.
	/// The first argument is the timestamp of the line, the second argument is the decoded line.
	Custom(fn(DateTime<Utc>, String)),
	/// Prints the line to stdout.
	Print,
	/// Prints with timestamp
	PrintWithTime,
}

pub struct TailOpts<'a> {
	pub print_type: PrintType,
	pub environment: &'a str,
	pub actor_id: Uuid,
	pub stream: LogStream,
	pub follow: bool,
	pub exit_on_ctrl_c: bool,
}

/// Reads the logs of an actor.
pub async fn tail(ctx: &ToolchainCtx, opts: TailOpts<'_>) -> Result<()> {
	let (stdout_fetched_tx, stdout_fetched_rx) = watch::channel(false);
	let (stderr_fetched_tx, stderr_fetched_rx) = watch::channel(false);

	let exit_on_ctrl_c = opts.exit_on_ctrl_c;
	
	tokio::select! {
		result = tail_streams(ctx, &opts, stdout_fetched_tx, stderr_fetched_tx) => result,
		result = poll_actor_state(ctx, &opts, stdout_fetched_rx, stderr_fetched_rx) => result,
		_ = signal::ctrl_c(), if exit_on_ctrl_c => {
			Ok(())
		}
	}
}

/// Reads the streams of an actor's logs.
async fn tail_streams(
	ctx: &ToolchainCtx,
	opts: &TailOpts<'_>,
	stdout_fetched_tx: watch::Sender<bool>,
	stderr_fetched_tx: watch::Sender<bool>,
) -> Result<()> {
	// TODO: Update ot use ActorsQueryLogStream::All
	tokio::try_join!(
		tail_stream(
			ctx,
			&opts,
			models::ActorsQueryLogStream::StdOut,
			stdout_fetched_tx
		),
		tail_stream(
			ctx,
			&opts,
			models::ActorsQueryLogStream::StdErr,
			stderr_fetched_tx
		),
	)
	.map(|_| ())
}

/// Reads a specific stream of an actor's log.
async fn tail_stream(
	ctx: &ToolchainCtx,
	opts: &TailOpts<'_>,
	stream: models::ActorsQueryLogStream,
	log_fetched_tx: watch::Sender<bool>,
) -> Result<()> {
	let mut watch_index: Option<String> = None;
	let mut first_batch_fetched = false;

	// Check if this stream is intended to be polled. If not, sleep indefinitely so the other
	// future doesn't exit.
	match (&opts.stream, stream) {
		(LogStream::All, _) => {}
		(LogStream::StdOut, models::ActorsQueryLogStream::StdOut) => {}
		(LogStream::StdErr, models::ActorsQueryLogStream::StdErr) => {}
		_ => {
			// Notify poll_actor_state
			log_fetched_tx.send(true).ok();

			// Do nothing
			return Ok(());
		}
	}

	loop {
		let res = apis::actors_logs_api::actors_logs_get(
			&ctx.openapi_config_cloud,
			stream,
			&serde_json::to_string(&vec![opts.actor_id])?,
			Some(&ctx.project.name_id),
			Some(opts.environment),
			None,
			None,
			None,
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
			let Result::Ok(ts) = ts.parse::<DateTime<Utc>>() else {
				eprintln!("Failed to parse timestamp: {ts} for line {line}");
				continue;
			};
			let decoded_line = match STANDARD.decode(line) {
				Result::Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
				Err(_) => {
					eprintln!("Failed to decode base64: {line}");
					continue;
				}
			};


			match &opts.print_type {
				PrintType::Custom(callback) => {
					(callback)(ts, decoded_line);
				}
				PrintType::Print => {
					println!("{decoded_line}");
				}
				PrintType::PrintWithTime => {
					println!("{ts} {decoded_line}");
				}
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
	ctx: &ToolchainCtx,
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

		let res = apis::actors_api::actors_get(
			&ctx.openapi_config_cloud,
			&opts.actor_id.to_string(),
			Some(&ctx.project.name_id),
			Some(opts.environment),
			None,
		)
		.await
		.map_err(|err| anyhow!("Failed to poll actor: {err}"))?;

		if res.actor.destroyed_at.is_some() {
			match opts.print_type {
				PrintType::Custom(_cb) => {}
				_ => {
					println!("Actor finished");
				}
			}
			return Ok(());
		}
	}
}
