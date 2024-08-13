use chirp_workflow::prelude::*;
use std::collections::HashMap;
use strum::FromRepr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromRepr)]
pub enum BuildKind {
	DockerImage = 0,
	OciBundle = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromRepr)]
pub enum BuildCompression {
	None = 0,
	Lz4 = 1,
}

#[derive(Debug)]
pub struct Build {
	pub build_id: Uuid,
	pub game_id: Uuid,
	pub upload_id: Uuid,
	pub display_name: String,
	pub image_tag: String,
	pub create_ts: i64,
	pub kind: BuildKind,
	pub compression: BuildCompression,
	pub tags: HashMap<String, String>,
}
