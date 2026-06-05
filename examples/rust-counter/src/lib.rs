use std::io::Cursor;

use anyhow::{Result, anyhow};
use rivetkit_core::{
	ActorConfig, ActorEvent, ActorFactory, ActorStart, CoreRegistry, RequestSaveOpts,
	SerializeStateReason, StateDelta,
};
use serde_json::{Value as JsonValue, json};

pub const ACTOR_NAME: &str = "counter";

pub fn registry() -> CoreRegistry {
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, counter_factory());
	registry
}

pub fn counter_factory() -> ActorFactory {
	ActorFactory::new(
		ActorConfig {
			name: Some(ACTOR_NAME.to_owned()),
			..ActorConfig::default()
		},
		|start| Box::pin(run_counter(start)),
	)
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
		.unwrap_or_default();

	while let Some(event) = events.recv().await {
		match event {
			ActorEvent::Action {
				name, args, reply, ..
			} => match name.as_str() {
				"increment" => {
					let amount = decode_increment_amount(&args).unwrap_or(1);
					count += amount;
					ctx.request_save(RequestSaveOpts::default());
					reply.send(Ok(encode_json(&json!(count))?));
				}
				"get" => {
					reply.send(Ok(encode_json(&json!(count))?));
				}
				other => {
					reply.send(Err(anyhow!("unknown action `{other}`")));
				}
			},
			ActorEvent::SerializeState { reason, reply } => match reason {
				SerializeStateReason::Save | SerializeStateReason::Inspector => {
					reply.send(Ok(vec![StateDelta::ActorState(encode_count(count)?)]));
				}
			},
			ActorEvent::RunGracefulCleanup { reply, .. } => {
				ctx.save_state(vec![StateDelta::ActorState(encode_count(count)?)])
					.await?;
				reply.send(Ok(()));
			}
			ActorEvent::HttpRequest { reply, .. } => {
				reply.send(Err(anyhow!("http requests are not handled")));
			}
			ActorEvent::QueueSend { reply, .. } => {
				reply.send(Err(anyhow!("queues are not handled")));
			}
			ActorEvent::WebSocketOpen { reply, .. } => {
				reply.send(Err(anyhow!("websockets are not handled")));
			}
			ActorEvent::ConnectionPreflight { reply, .. } => {
				reply.send(Ok(()));
			}
			ActorEvent::ConnectionOpen { reply, .. } => {
				reply.send(Ok(()));
			}
			ActorEvent::ConnectionClosed { .. } => {}
			ActorEvent::SubscribeRequest { reply, .. } => {
				reply.send(Err(anyhow!("subscriptions are not handled")));
			}
			ActorEvent::DisconnectConn { reply, .. } => {
				reply.send(Ok(()));
			}
			ActorEvent::WorkflowHistoryRequested { reply } => {
				reply.send(Ok(None));
			}
			ActorEvent::WorkflowReplayRequested { reply, .. } => {
				reply.send(Ok(None));
			}
		}
	}

	Ok(())
}

fn decode_increment_amount(args: &[u8]) -> Option<i64> {
	let value: JsonValue = ciborium::from_reader(Cursor::new(args)).ok()?;
	if let Some(amount) = value.as_i64() {
		return Some(amount);
	}
	value
		.as_array()
		.and_then(|items| items.first())
		.and_then(JsonValue::as_i64)
}

fn encode_count(count: i64) -> Result<Vec<u8>> {
	encode_json(&json!({ "count": count }))
}

fn decode_count(bytes: &[u8]) -> Result<i64> {
	if bytes.is_empty() {
		return Ok(0);
	}
	let value: JsonValue = ciborium::from_reader(Cursor::new(bytes))?;
	Ok(value
		.get("count")
		.and_then(JsonValue::as_i64)
		.unwrap_or_default())
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ciborium::into_writer(value, &mut out)?;
	Ok(out)
}
