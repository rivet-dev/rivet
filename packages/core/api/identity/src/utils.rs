use api_helper::ctx::Ctx;
use proto::common;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// Used to get the game id when the game user has not been made yet
pub async fn get_game_id(ctx: &Ctx<Auth>) -> GlobalResult<common::Uuid> {
	let namespace_id = if let Some(ns_dev_ent) = ctx.auth().game_ns_dev_option()? {
		ns_dev_ent.namespace_id
	} else {
		ctx.auth().game_ns(ctx).await?.namespace_id
	};

	let namespace_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()]
	})
	.await?;

	let namespace = unwrap!(namespace_res.namespaces.first()).clone();

	Ok(unwrap!(namespace.game_id))
}

// Used to get the game id when the game user has not been made yet
pub async fn get_namespace_id(ctx: &Ctx<Auth>) -> GlobalResult<common::Uuid> {
	let namespace_id = if let Some(ns_dev_ent) = ctx.auth().game_ns_dev_option()? {
		ns_dev_ent.namespace_id
	} else {
		ctx.auth().game_ns(ctx).await?.namespace_id
	};

	Ok(namespace_id.into())
}
