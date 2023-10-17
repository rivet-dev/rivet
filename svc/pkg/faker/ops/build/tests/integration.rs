use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
	})
	.await
	.unwrap();

	let images = vec![
		faker::build::Image::HangIndefinitely,
		faker::build::Image::MmLobbyAutoReady,
		faker::build::Image::FailImmediately,
		faker::build::Image::MmPlayerConnect,
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
