use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "faker-build")]
async fn handle(
	ctx: OperationContext<faker::build::Request>,
) -> GlobalResult<faker::build::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();

	// Determine image name
	//
	// These are built in `bin/runtime_docker_builds/`
	let build_kind = if ctx.image == faker::build::Image::HangIndefinitely as i32 {
		"test-hang-indefinitely"
	} else if ctx.image == faker::build::Image::MmLobbyAutoReady as i32 {
		"test-mm-lobby-ready"
	} else if ctx.image == faker::build::Image::FailImmediately as i32 {
		"test-fail-immediately"
	} else {
		internal_panic!("invalid image");
	};

	let create_res = op!([ctx] build_create {
		game_id: Some(game_id.into()),
		display_name: util::faker::display_name(),
		default_build_kind: Some(build_kind.into()),
		..Default::default()
	})
	.await?;
	let build_id = internal_unwrap!(create_res.build_id).as_uuid();

	Ok(faker::build::Response {
		build_id: Some(build_id.into()),
	})
}
