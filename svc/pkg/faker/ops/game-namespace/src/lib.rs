use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "faker-game-namespace")]
async fn handle(
	ctx: OperationContext<faker::game_namespace::Request>,
) -> GlobalResult<faker::game_namespace::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();
	let version_id = internal_unwrap!(ctx.version_id).as_uuid();

	let create_ns_res = op!([ctx] game_namespace_create {
		game_id: Some(game_id.into()),
		display_name: if ctx.override_display_name.is_empty() {
			util::faker::display_name()
		} else {
			ctx.override_display_name.clone()
		},
		version_id: Some(version_id.into()),
		name_id: if ctx.override_name_id.is_empty() {
			util::faker::ident()
		} else {
			ctx.override_name_id.clone()
		},
	})
	.await
	.unwrap();
	let namespace_id = internal_unwrap!(create_ns_res.namespace_id).as_uuid();

	op!([ctx] cloud_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	Ok(faker::game_namespace::Response {
		namespace_id: create_ns_res.namespace_id,
	})
}
