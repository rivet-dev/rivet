use std::fmt;

use anyhow::{Context, Result, bail};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};
use universaldb::utils::keys::{ACTOR, DATA};
use universalpubsub::PublishOpts;
use uuid::Uuid;
use vbare::OwnedVersionedData;

use crate::keys;

pub const ACTOR_LIFECYCLE_SUBJECT: &str = "pegboard.actor.lifecycle";
pub const RESTORE_SUSPENSION_REASON: &str = "sqlite_restore";
pub const RESTORE_WS_CLOSE_REASON: &str = "actor.restore_in_progress";
pub const RESTORE_WS_CLOSE_CODE: u16 = 1012;
pub const RESTORE_HTTP_RETRY_AFTER_SECONDS: &str = "30";
const ACTOR_SUSPENSION_VERSION: u16 = 1;
const ACTOR_LIFECYCLE_MESSAGE_VERSION: u16 = 1;
const SUSPENSION_KEY_SEGMENT: &str = "suspension";

/// Actor suspension is a pegboard-level traffic gate used by destructive
/// SQLite restore. `suspend_actor` persists the gate before broadcasting a
/// lifecycle message, so envoys that miss the broadcast still reject new
/// tunnel traffic after checking storage. Existing websocket routes are
/// closed with `1012 actor.restore_in_progress`; HTTP routes receive
/// `503 Retry-After: 30`. `resume_actor` clears the gate and broadcasts a
/// resume message after the restore operation reaches `Completed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSuspension {
	pub actor_id: String,
	pub reason: String,
	pub op_id: Uuid,
	pub suspended_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorLifecycleMessage {
	Suspended(ActorSuspension),
	Resumed { actor_id: String },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ActorLifecycleSubject;

impl fmt::Display for ActorLifecycleSubject {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(ACTOR_LIFECYCLE_SUBJECT)
	}
}

impl universalpubsub::Subject for ActorLifecycleSubject {
	fn root<'a>() -> Option<std::borrow::Cow<'a, str>> {
		Some(std::borrow::Cow::Borrowed(ACTOR_LIFECYCLE_SUBJECT))
	}

	fn as_str(&self) -> Option<&str> {
		Some(ACTOR_LIFECYCLE_SUBJECT)
	}
}

enum VersionedActorSuspension {
	V1(ActorSuspension),
}

impl OwnedVersionedData for VersionedActorSuspension {
	type Latest = ActorSuspension;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid actor suspension version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedActorLifecycleMessage {
	V1(ActorLifecycleMessage),
}

impl OwnedVersionedData for VersionedActorLifecycleMessage {
	type Latest = ActorLifecycleMessage;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid actor lifecycle message version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub async fn suspend_actor(
	udb: &universaldb::Database,
	ups: &universalpubsub::PubSub,
	actor_id: String,
	reason: &str,
	op_id: Uuid,
) -> Result<ActorSuspension> {
	let suspension = ActorSuspension {
		actor_id: actor_id.clone(),
		reason: reason.to_string(),
		op_id,
		suspended_at_ms: util::timestamp::now(),
	};
	let encoded = encode_suspension(suspension.clone())?;
	let key = suspension_key(&actor_id);

	udb.run(move |tx| {
		let key = key.clone();
		let encoded = encoded.clone();
		async move {
			tx.informal().set(&key, &encoded);
			Ok(())
		}
	})
	.custom_instrument(tracing::info_span!("actor_suspend_tx"))
	.await?;

	publish_lifecycle(ups, ActorLifecycleMessage::Suspended(suspension.clone())).await?;
	Ok(suspension)
}

pub async fn resume_actor(
	udb: &universaldb::Database,
	ups: &universalpubsub::PubSub,
	actor_id: String,
) -> Result<()> {
	let key = suspension_key(&actor_id);
	udb.run(move |tx| {
		let key = key.clone();
		async move {
			tx.informal().clear(&key);
			Ok(())
		}
	})
	.custom_instrument(tracing::info_span!("actor_resume_tx"))
	.await?;

	publish_lifecycle(ups, ActorLifecycleMessage::Resumed { actor_id }).await
}

pub async fn read_suspension(
	udb: &universaldb::Database,
	actor_id: &str,
) -> Result<Option<ActorSuspension>> {
	let key = suspension_key(actor_id);
	udb.run(move |tx| {
		let key = key.clone();
		async move {
			let Some(raw) = tx.informal().get(&key, Snapshot).await? else {
				return Ok(None);
			};
			Ok(Some(decode_suspension(&raw)?))
		}
	})
	.custom_instrument(tracing::info_span!("actor_read_suspension_tx"))
	.await
}

pub async fn is_suspended(udb: &universaldb::Database, actor_id: &str) -> Result<bool> {
	let key = suspension_key(actor_id);
	udb.run(move |tx| {
		let key = key.clone();
		async move { tx.informal().get(&key, Serializable).await.map(|x| x.is_some()) }
	})
	.custom_instrument(tracing::info_span!("actor_is_suspended_tx"))
	.await
	.map_err(Into::into)
}

pub fn encode_lifecycle_message(message: ActorLifecycleMessage) -> Result<Vec<u8>> {
	VersionedActorLifecycleMessage::wrap_latest(message)
		.serialize_with_embedded_version(ACTOR_LIFECYCLE_MESSAGE_VERSION)
		.context("encode actor lifecycle message")
}

pub fn decode_lifecycle_message(payload: &[u8]) -> Result<ActorLifecycleMessage> {
	VersionedActorLifecycleMessage::deserialize_with_embedded_version(payload)
		.context("decode actor lifecycle message")
}

fn encode_suspension(suspension: ActorSuspension) -> Result<Vec<u8>> {
	VersionedActorSuspension::wrap_latest(suspension)
		.serialize_with_embedded_version(ACTOR_SUSPENSION_VERSION)
		.context("encode actor suspension")
}

fn decode_suspension(payload: &[u8]) -> Result<ActorSuspension> {
	VersionedActorSuspension::deserialize_with_embedded_version(payload)
		.context("decode actor suspension")
}

async fn publish_lifecycle(
	ups: &universalpubsub::PubSub,
	message: ActorLifecycleMessage,
) -> Result<()> {
	ups.publish(
		ActorLifecycleSubject,
		&encode_lifecycle_message(message)?,
		PublishOpts::broadcast(),
	)
	.await
	.context("publish actor lifecycle message")
}

fn suspension_key(actor_id: &str) -> Vec<u8> {
	keys::subspace()
		.subspace(&(ACTOR, DATA, SUSPENSION_KEY_SEGMENT, actor_id))
		.bytes()
		.to_vec()
}
