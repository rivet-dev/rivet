use chirp_workflow::prelude::*;
use futures_util::FutureExt;

use crate::types::{BuildDeliveryMethod, Pool, Provider};

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub cluster_id: Uuid,
	pub name_id: String,
	pub owner_team_id: Option<Uuid>,
}

#[workflow]
pub async fn cluster(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	ctx.activity(InsertDbInput {
		cluster_id: input.cluster_id,
		name_id: input.name_id.clone(),
		owner_team_id: input.owner_team_id,
	})
	.await?;

	ctx.msg(CreateComplete {})
		.tag("cluster_id", input.cluster_id)
		.send()
		.await?;

	ctx.repeat(|ctx| {
		let cluster_id = input.cluster_id;

		async move {
			match ctx.listen::<Main>().await? {
				Main::GameLink(sig) => {
					ctx.activity(GameLinkInput {
						cluster_id,
						game_id: sig.game_id,
					})
					.await?;

					ctx.msg(GameLinkComplete {})
						.tag("cluster_id", cluster_id)
						.send()
						.await?;
				}
				Main::DatacenterCreate(sig) => {
					ctx.workflow(crate::workflows::datacenter::Input {
						cluster_id,
						datacenter_id: sig.datacenter_id,
						name_id: sig.name_id,
						display_name: sig.display_name,

						provider: sig.provider,
						provider_datacenter_id: sig.provider_datacenter_id,
						provider_api_token: sig.provider_api_token,

						pools: sig.pools,

						build_delivery_method: sig.build_delivery_method,
						prebakes_enabled: sig.prebakes_enabled,
					})
					.tag("datacenter_id", sig.datacenter_id)
					.dispatch()
					.await?;
				}
			}

			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	cluster_id: Uuid,
	name_id: String,
	owner_team_id: Option<Uuid>,
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.clusters (
			cluster_id,
			name_id,
			owner_team_id,
			create_ts
		)
		VALUES ($1, $2, $3, $4)
		",
		input.cluster_id,
		&input.name_id,
		input.owner_team_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}

#[message("cluster_create_complete")]
pub struct CreateComplete {}

#[signal("cluster_game_link")]
pub struct GameLink {
	pub game_id: Uuid,
}

#[signal("cluster_datacenter_create")]
pub struct DatacenterCreate {
	pub datacenter_id: Uuid,
	pub name_id: String,
	pub display_name: String,

	pub provider: Provider,
	pub provider_datacenter_id: String,
	pub provider_api_token: Option<String>,

	pub pools: Vec<Pool>,

	pub build_delivery_method: BuildDeliveryMethod,
	pub prebakes_enabled: bool,
}
join_signal!(Main {
	GameLink,
	DatacenterCreate,
});

#[message("cluster_game_link_complete")]
pub struct GameLinkComplete {}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GameLinkInput {
	cluster_id: Uuid,
	game_id: Uuid,
}

#[activity(GameLinkActivity)]
async fn game_link(ctx: &ActivityCtx, input: &GameLinkInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.games (
			game_id,
			cluster_id
		)
		VALUES ($1, $2)
		",
		input.game_id,
		input.cluster_id,
	)
	.await?;

	Ok(())
}
