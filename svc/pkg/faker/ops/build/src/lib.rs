use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-build")]
async fn handle(
	ctx: OperationContext<faker::build::Request>,
) -> GlobalResult<faker::build::Response> {
	// Determine image name
	//
	// These are built in `bin/runtime_docker_builds/`
	let build_kind = match unwrap!(backend::faker::Image::from_i32(ctx.image)) {
		backend::faker::Image::FailImmediately => "test-fail-immediately",
		backend::faker::Image::HangIndefinitely => "test-hang-indefinitely",
		backend::faker::Image::MmLobbyAutoReady => "test-mm-lobby-ready",
		backend::faker::Image::MmLobbyEcho => "test-mm-lobby-echo",
		backend::faker::Image::MmPlayerConnect => "test-mm-player-connect",
		backend::faker::Image::DsEcho => "test-ds-echo",
	};

	let create_res = op!([ctx] build_create {
		game_id: ctx.game_id,
		env_id: ctx.env_id,
		display_name: util::faker::display_name(),
		default_build_kind: Some(build_kind.into()),
		..Default::default()
	})
	.await?;
	let build_id = unwrap_ref!(create_res.build_id).as_uuid();

	Ok(faker::build::Response {
		build_id: Some(build_id.into()),
	})
}
