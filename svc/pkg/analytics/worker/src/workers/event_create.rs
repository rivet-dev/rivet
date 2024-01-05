use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[derive(clickhouse::Row, serde::Serialize, Debug)]
struct Event {
	ts: i64,
	#[serde(with = "clickhouse::serde::uuid")]
	event_id: Uuid,
	#[serde(with = "clickhouse::serde::uuid")]
	ray_id: Uuid,
	name: String,
	properties_raw: String,
}

#[worker(name = "analytics-event-create")]
async fn worker(ctx: &OperationContext<analytics::msg::event_create::Message>) -> GlobalResult<()> {
	let ray_id = ctx.ray_id();

	let client = rivet_pools::utils::clickhouse::client()?
		.with_user("chirp")
		.with_password(util::env::read_secret(&["clickhouse", "users", "chirp", "password"]).await?)
		.with_database("db_analytics");

	let user_ids_proto = ctx
		.events
		.iter()
		.flat_map(|x| x.user_id)
		.map(|x| x.as_uuid())
		.collect::<HashSet<Uuid>>()
		.into_iter()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();

	// Build events
	let mut insert = client.insert("events")?;
	for req_event in &ctx.events {
		let event = build_event(req_event.ts.unwrap_or(ctx.ts()), ray_id, req_event)?;
		insert.write(&event).await?;
	}
	insert.end().await?;

	Ok(())
}

fn build_event(
	ts: i64,
	ray_id: Uuid,
	req_event: &analytics::msg::event_create::Event,
) -> GlobalResult<Event> {
	let mut properties = HashMap::<String, Box<serde_json::value::RawValue>>::new();

	// Insert provided properties
	if let Some(properties_json) = &req_event.properties_json {
		let req_properties = serde_json::from_str::<
			HashMap<String, Box<serde_json::value::RawValue>>,
		>(properties_json)?;
		properties.extend(req_properties.into_iter());
	}

	// Insert common legacy properties
	if let Some(user_id) = req_event.user_id {
		serialize_prop(&mut properties, "user_id", user_id.as_uuid())?;
	}
	if let Some(ns_id) = req_event.namespace_id {
		serialize_prop(&mut properties, "namespace_id", ns_id.as_uuid())?;
	}

	Ok(Event {
		ts,
		// TODO: Pass the event ID when the analytics event is created to
		// prevent dupes
		event_id: Uuid::new_v4(),
		ray_id,
		name: req_event.name.clone(),
		properties_raw: serde_json::to_string(&properties)?,
	})
}

fn serialize_prop(
	properties: &mut HashMap<String, Box<serde_json::value::RawValue>>,
	key: impl ToString,
	value: impl serde::Serialize,
) -> GlobalResult<()> {
	properties.insert(
		key.to_string(),
		serde_json::value::RawValue::from_string(serde_json::to_string(&value)?)?,
	);
	Ok(())
}
