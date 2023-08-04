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
async fn worker(ctx: OperationContext<analytics::msg::event_create::Message>) -> GlobalResult<()> {
	let ray_id = ctx.ray_id();

	let client = clickhouse::Client::default()
		.with_url("http://http.clickhouse.service.consul:8123")
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

	// Fetch the user's current presence to enrich the event
	let user_presences = op!([ctx] user_presence_get {
		user_ids: user_ids_proto.clone(),
	})
	.await?;

	// Build events
	let mut insert = client.insert("events")?;
	for req_event in &ctx.events {
		let event = build_event(
			req_event.ts.unwrap_or(ctx.ts()),
			ray_id,
			req_event,
			&user_presences.users,
		)?;
		insert.write(&event).await?;
	}
	insert.end().await?;

	Ok(())
}

fn build_event(
	ts: i64,
	ray_id: Uuid,
	req_event: &analytics::msg::event_create::Event,
	user_presences: &[user_presence::get::UserPresenceEntry],
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

	// Insert user presence
	if let Some(user_presence) = user_presences
		.iter()
		.find(|x| x.user_id == req_event.user_id)
		.and_then(|x| x.presence.as_ref())
	{
		serialize_prop(&mut properties, "presence_status", user_presence.status)?;
		if let Some(game_activity) = &user_presence.game_activity {
			serialize_prop(
				&mut properties,
				"presence_game_id",
				internal_unwrap!(game_activity.game_id).as_uuid(),
			)?;
			// TODO: Add back when Serde decoding is fixed (RIV-2278)
			// if let Some(public_metadata) = &game_activity.public_metadata {
			// 	serialize_prop(
			// 		&mut properties,
			// 		"presence_game_public_metadata",
			// 		serde_json::from_str::<serde_json::Value>(public_metadata)?,
			// 	)?;
			// }
			// if let Some(friend_metadata) = &game_activity.friend_metadata {
			// 	serialize_prop(
			// 		&mut properties,
			// 		"presence_game_friend_metadata",
			// 		serde_json::from_str::<serde_json::Value>(friend_metadata)?,
			// 	)?;
			// }
		}
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
