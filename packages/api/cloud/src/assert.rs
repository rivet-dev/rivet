use api_helper::ctx::Ctx;
use proto::backend;
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that a given user ID is registered.
pub async fn user_registered(ctx: &OperationContext<()>, user_id: Uuid) -> GlobalResult<()> {
	// If the user has at least one identity they are considered registered
	let identity = op!([ctx] user_identity_get {
		user_ids: vec![user_id.into()]
	})
	.await?;

	let identities = &unwrap_ref!(identity.users.first()).identities;
	ensure_with!(!identities.is_empty(), IDENTITY_NOT_REGISTERED);

	Ok(())
}

/// Validates that a game's version exists and belongs to the given game ID.
///
/// Throws `NotFound` if version does not exist and `BadRequest` if does not belong to the given
/// game.
pub async fn version_for_game(
	ctx: &Ctx<Auth>,
	game_id: Uuid,
	version_id: Uuid,
) -> GlobalResult<backend::game::Version> {
	let version_get_res = op!([ctx] game_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;

	let version_data = unwrap!(version_get_res.versions.first());
	let version_game_id = unwrap_ref!(version_data.game_id).as_uuid();

	ensure_eq!(version_game_id, game_id, "version does not belong to game");

	Ok(version_data.clone())
}

/// Validates that a game's namespace exists and belongs to the given game ID.
///
/// Throws `NotFound` if namespace does not exist and `BadRequest` if does not belong to the given
/// game.
pub async fn namespace_for_game(
	ctx: &Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
) -> GlobalResult<backend::game::Namespace> {
	let ns_get_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;

	let ns_data = unwrap!(ns_get_res.namespaces.first());
	let ns_game_id = unwrap_ref!(ns_data.game_id).as_uuid();

	ensure_eq!(ns_game_id, game_id, "namespace does not belong to game");

	Ok(ns_data.clone())
}
