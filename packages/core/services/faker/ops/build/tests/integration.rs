use chirp_worker::prelude::*;
use proto::backend::{self};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
	})
	.await
	.unwrap();

	let images = vec![
		backend::faker::Image::HangIndefinitely,
		backend::faker::Image::MmLobbyAutoReady,
		backend::faker::Image::FailImmediately,
		backend::faker::Image::MmPlayerConnect,
	];

	// Build all images in parallel
	for image in images {
		tracing::info!(?image, "building");
		op!([ctx] faker_build {
			game_id: game_res.game_id,
			image: image as i32,
		})
		.await
		.unwrap();
	}
}
