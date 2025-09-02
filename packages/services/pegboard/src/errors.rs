use gas::prelude::*;
use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("actor")]
pub enum Actor {
	#[error("not_found", "The actor does not exist.")]
	NotFound,

	#[error("namespace_not_found", "The namespace does not exist.")]
	NamespaceNotFound,

	#[error(
		"input_too_large",
		"Actor input too large.",
		"Input too large (max {max_size})."
	)]
	InputTooLarge { max_size: usize },

	#[error("empty_key", "Key label cannot be empty.")]
	EmptyKey,

	#[error(
		"key_too_large",
		"Key label too large.",
		"Key label too large (max {max_size} bytes): {key_preview}"
	)]
	KeyTooLarge {
		max_size: usize,
		key_preview: String,
	},

	#[error(
		"duplicate_key",
		"Actor key already in use.",
		"Actor key '{key}' already in use for actor '{existing_actor_id}'"
	)]
	DuplicateKey { key: String, existing_actor_id: Id },

	#[error("destroyed_during_creation", "Actor was destroyed during creation.")]
	DestroyedDuringCreation,

	#[error(
		"destroyed_while_waiting_for_ready",
		"Actor was destroyed while waiting for ready state."
	)]
	DestroyedWhileWaitingForReady,

	#[error(
		"key_reserved_in_different_datacenter",
		"Actor key is already reserved in a different datacenter. Either remove the datacenter constraint to automatically create this actor in the correct datacenter or provide the datacenter that matches.",
		"Actor key is already reserved in the datacenter '{datacenter_label}'. Either remove the datacenter constraint to automatically create this actor in the correct datacenter or provide the datacenter that matches."
	)]
	KeyReservedInDifferentDatacenter { datacenter_label: u16 },
}

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("runner")]
pub enum Runner {
	#[error("not_found", "The runner does not exist.")]
	NotFound,
}
