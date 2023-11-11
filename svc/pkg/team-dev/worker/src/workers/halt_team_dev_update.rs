use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "team-dev-halt-team-dev-update")]
async fn worker(ctx: &OperationContext<team_dev::msg::update::Message>) -> GlobalResult<()> {
	let team_id = unwrap_ref!(ctx.team_id).as_uuid();

	let team_dev_res = op!([ctx] team_dev_get {
		team_ids: vec![team_id.into()],
	})
	.await?;
	let dev_team = unwrap!(team_dev_res.teams.first());

	let status = unwrap!(backend::team::dev_team::DevStatus::from_i32(
		dev_team.status
	));
	tracing::info!(?status);

	if !dev_team.active {
		op!([ctx] team_dev_halt {
			team_ids: vec![team_id.into()],
		})
		.await?;
	}

	Ok(())
}
