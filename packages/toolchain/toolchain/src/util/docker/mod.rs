pub mod archive;
pub mod build;
pub mod push;
pub mod users;

use uuid::Uuid;

/// Generates a unique image tag for the image being pushed or built.
pub fn generate_unique_image_tag() -> String {
	format!("rivet-game:{}", Uuid::new_v4())
}
