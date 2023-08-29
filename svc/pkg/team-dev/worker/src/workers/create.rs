use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-dev-create")]
async fn worker(ctx: &OperationContext<team_dev::msg::create::Message>) -> GlobalResult<()> {
	let team_id = internal_unwrap!(ctx.team_id).as_uuid();

	// Get the team
	let team_res = op!([ctx] team_get {
		team_ids: vec![team_id.into()],
	})
	.await?;
	let team = internal_unwrap_owned!(team_res.teams.first(), "team not found");
	let owner_user_id = internal_unwrap_owned!(team.owner_user_id);

	let dev_team_res = op!([ctx] team_dev_get {
		team_ids: vec![team_id.into()],
	})
	.await?;
	if !dev_team_res.teams.is_empty() {
		// TODO: RIV-2281
		tracing::info!("team is already a dev team");
		return Ok(());
	}

	// Create stripe customer
	let stripe_customer_id = if util::env::is_billing_enabled() {
		// Get user's identities
		let identity_res = op!([ctx] user_identity_get {
			user_ids: vec![owner_user_id],
		})
		.await?;

		let email = {
			let user = identity_res.users.first();
			let user = internal_unwrap!(user);
			let email_ident = user.identities.iter().find(|identity| {
				matches!(
					identity.kind,
					Some(backend::user_identity::identity::Kind::Email(_))
				)
			});

			if let Some(backend::user_identity::Identity {
				kind: Some(backend::user_identity::identity::Kind::Email(email_ident)),
			}) = email_ident
			{
				Some(email_ident.email.clone())
			} else {
				None
			}
		};

		// TODO: Redo customer creation with stripe
		let stripe_customer_id = String::new();

		Some(stripe_customer_id)
	} else {
		None
	};

	// Create the dev team
	let crdb = ctx.crdb("db-team-dev").await?;
	sqlx::query(indoc!(
		"
		INSERT INTO dev_teams (team_id, create_ts, stripe_customer_id)
		VALUES ($1, $2, $3)
		"
	))
	.bind(team_id)
	.bind(ctx.ts())
	.bind(stripe_customer_id)
	.execute(&crdb)
	.await?;

	msg!([ctx] team::msg::update(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.dev.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
