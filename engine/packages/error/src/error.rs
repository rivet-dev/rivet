use crate::INTERNAL_ERROR;
use crate::schema::RivetErrorSchema;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::OnceLock};

static EXPOSE_INTERNAL_ERRORS: OnceLock<bool> = OnceLock::new();

fn expose_internal_errors() -> bool {
	*EXPOSE_INTERNAL_ERRORS
		.get_or_init(|| matches!(std::env::var("RIVET_EXPOSE_ERRORS").as_deref(), Ok("1")))
}

/// Identifies the actor that was handling work when an error was produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorSpecifier {
	pub actor_id: String,
	pub generation: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,
}

impl ActorSpecifier {
	pub fn new(actor_id: impl Into<String>, generation: u64) -> Self {
		Self {
			actor_id: actor_id.into(),
			generation,
			key: None,
		}
	}

	pub fn with_key(mut self, key: impl Into<String>) -> Self {
		self.key = Some(key.into());
		self
	}
}

impl fmt::Display for ActorSpecifier {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self.key {
			Some(key) => write!(
				f,
				"actor {} generation {} key {}",
				self.actor_id, self.generation, key
			),
			None => write!(f, "actor {} generation {}", self.actor_id, self.generation),
		}
	}
}

impl std::error::Error for ActorSpecifier {}

#[derive(Debug, Clone)]
pub enum RivetErrorKind {
	Static(&'static RivetErrorSchema),
	Dynamic {
		group: String,
		code: String,
		default_message: String,
	},
}

impl RivetErrorKind {
	pub fn group(&self) -> &str {
		match self {
			Self::Static(schema) => schema.group,
			Self::Dynamic { group, .. } => group,
		}
	}

	pub fn code(&self) -> &str {
		match self {
			Self::Static(schema) => schema.code,
			Self::Dynamic { code, .. } => code,
		}
	}

	pub fn default_message(&self) -> &str {
		match self {
			Self::Static(schema) => schema.default_message,
			Self::Dynamic {
				default_message, ..
			} => default_message,
		}
	}

	pub fn schema(&self) -> Option<&'static RivetErrorSchema> {
		match self {
			Self::Static(schema) => Some(schema),
			Self::Dynamic { .. } => None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct RivetError {
	pub kind: RivetErrorKind,
	pub meta: Option<Box<serde_json::value::RawValue>>,
	pub message: Option<String>,
	pub actor: Option<ActorSpecifier>,
}

impl RivetError {
	pub fn extract(error: &anyhow::Error) -> Self {
		// `anyhow::Error::downcast_ref` walks both the chain and any
		// `.context(...)` wrappers, so this finds an `ActorSpecifier` no matter
		// where it was attached.
		let actor = error.downcast_ref::<ActorSpecifier>().cloned();
		let mut extracted = error
			.chain()
			.find_map(|x| x.downcast_ref::<Self>())
			.cloned()
			.unwrap_or_else(|| INTERNAL_ERROR.build_internal(error));
		if extracted.actor.is_none() {
			extracted.actor = actor;
		}
		extracted
	}

	pub(crate) fn build_internal(error: &anyhow::Error) -> Self {
		let error_string = format!("{:?}", error);
		let meta_json = serde_json::json!({
			"error": error_string
		});
		let meta = serde_json::value::to_raw_value(&meta_json).ok();

		Self {
			kind: RivetErrorKind::Static(&INTERNAL_ERROR),
			meta,
			message: expose_internal_errors().then(|| format!("Internal error: {}", error)),
			actor: None,
		}
	}

	pub fn group(&self) -> &str {
		self.kind.group()
	}

	pub fn code(&self) -> &str {
		self.kind.code()
	}

	pub fn message(&self) -> &str {
		self.message
			.as_deref()
			.unwrap_or_else(|| self.kind.default_message())
	}

	pub fn metadata(&self) -> Option<serde_json::Value> {
		self.meta
			.as_ref()
			.and_then(|raw| serde_json::from_str(raw.get()).ok())
	}

	pub fn actor(&self) -> Option<&ActorSpecifier> {
		self.actor.as_ref()
	}

	pub fn with_actor(mut self, actor: ActorSpecifier) -> Self {
		self.actor = Some(actor);
		self
	}

	pub fn schema(&self) -> Option<&'static RivetErrorSchema> {
		self.kind.schema()
	}
}

impl fmt::Display for RivetError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}: {}", self.code(), self.message())
	}
}

impl std::error::Error for RivetError {}

impl Serialize for RivetError {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeStruct;

		let field_count =
			3 + usize::from(self.meta.is_some()) + usize::from(self.actor.is_some());
		let mut state = serializer.serialize_struct("RivetError", field_count)?;

		state.serialize_field("group", self.group())?;
		state.serialize_field("code", self.code())?;
		state.serialize_field("message", self.message())?;

		if let Some(meta) = &self.meta {
			state.serialize_field("meta", meta)?;
		}

		if let Some(actor) = &self.actor {
			state.serialize_field("actor", actor)?;
		}

		state.end()
	}
}
