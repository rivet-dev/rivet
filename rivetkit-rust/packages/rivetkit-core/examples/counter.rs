//! Minimal counter actor built directly against `rivetkit-core`.
//!
//! Most applications should use the higher-level `rivetkit` crate. This
//! example shows the lower-level receive-loop API exposed by `rivetkit-core`.

use std::io::Cursor;

use anyhow::{Result, anyhow};
use ciborium::{from_reader, into_writer};
use rivetkit_core::{
	ActorConfig, ActorEvent, ActorFactory, ActorStart, CoreRegistry, RequestSaveOpts,
	SerializeStateReason, StateDelta,
};

fn encode_count(count: i64) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	into_writer(&count, &mut out)?;
	Ok(out)
}

fn decode_count(bytes: &[u8]) -> Result<i64> {
	if bytes.is_empty() {
		return Ok(0);
	}
	from_reader(Cursor::new(bytes)).map_err(Into::into)
}

async fn run_counter(start: ActorStart) -> Result<()> {
	let ActorStart {
		ctx,
		snapshot,
		mut events,
		..
	} = start;
	let mut count = snapshot
		.as_deref()
		.map(decode_count)
		.transpose()?
		.unwrap_or(0);
	let mut dirty = false;

	while let Some(event) = events.recv().await {
		match event {
			ActorEvent::Action {
				name, args, reply, ..
			} => match name.as_str() {
				"increment" => {
					let delta = decode_count(&args).unwrap_or(1);
					count += delta;
					dirty = true;
					ctx.request_save(RequestSaveOpts::default());
					reply.send(Ok(encode_count(count)?));
				}
				"get" => {
					reply.send(Ok(encode_count(count)?));
				}
				other => {
					reply.send(Err(anyhow!("unknown action `{other}`")));
				}
			},
			ActorEvent::SerializeState {
				reason: SerializeStateReason::Save,
				reply,
			} => {
				reply.send(build_deltas(count, &mut dirty));
			}
			ActorEvent::RunGracefulCleanup { reply, .. } => {
				ctx.save_state(build_deltas(count, &mut dirty)?).await?;
				reply.send(Ok(()));
				break;
			}
			_ => {}
		}
	}

	Ok(())
}

fn build_deltas(count: i64, dirty: &mut bool) -> Result<Vec<StateDelta>> {
	if !*dirty {
		return Ok(Vec::new());
	}

	*dirty = false;
	Ok(vec![StateDelta::ActorState(encode_count(count)?)])
}

fn counter_factory() -> ActorFactory {
	ActorFactory::new(
		ActorConfig {
			name: Some("counter".to_owned()),
			..ActorConfig::default()
		},
		|start| Box::pin(run_counter(start)),
	)
}

#[tokio::main]
async fn main() -> Result<()> {
	let mut registry = CoreRegistry::new();
	registry.register("counter", counter_factory());
	registry.serve().await
}
