use chirp_workflow::prelude::*;

pub mod client;
pub mod datacenter;

#[signal("pegboard_prewarm_image")]
pub struct PrewarmImage {
	pub image_id: Uuid,
	pub image_artifact_url_stub: String,
}
