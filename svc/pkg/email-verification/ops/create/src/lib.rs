use email_address_parser::EmailAddress;
use proto::backend::pkg::*;
use rand::Rng;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "email-verification-create")]
async fn handle(
	ctx: OperationContext<email_verification::create::Request>,
) -> GlobalResult<email_verification::create::Response> {
	let crdb = ctx.crdb().await?;

	let email_parse = EmailAddress::parse(&ctx.email, None);
	let email = unwrap_ref!(email_parse);

	// Combine the email parts together and make the entire email lowercase
	let email = format!("{}@{}", email.get_local_part(), email.get_domain()).to_lowercase();

	// Save verification code
	let verification_id = Uuid::new_v4();
	let code = gen_code();
	let expire_ts = ctx.ts() + util::duration::minutes(15);
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_email_verification.verifications (
			verification_id,
			email,
			code,
			create_ts,
			expire_ts
		)
		VALUES ($1, $2, $3, $4, $5)
		",
		verification_id,
		&email,
		&code,
		ctx.ts(),
		expire_ts,
	)
	.await?;

	let (template_id, dynamic_template_data) = if let Some(game_id) = ctx.game_id {
		let games_res = op!([ctx] game_get {
			game_ids: vec![game_id],
		})
		.await?;
		let game = unwrap_with!(games_res.games.first(), GAME_NOT_FOUND);

		(
			"d-a742c54153a6436694516fc58cb6eabf".to_string(),
			json!({
				"game_display_name": game.display_name.clone(),
				"game_logo_url": util::route::game_logo(game),
				"verification_code": code,
			}),
		)
	} else {
		(
			"d-054b232e5c7f46f68df3b7d094b74dfb".to_string(),
			json!({
				"verificationCode": code,
			}),
		)
	};

	// Send email
	op!([ctx] email_send {
		from_address: Some(email::send::Address {
			email: "hello@rivet.gg".into(),
			..Default::default()
		}),
		messages: vec![
			email::send::Message {
				to_addresses: vec![email::send::Address {
					email: ctx.email.clone(),
					..Default::default()
				}],
				dynamic_template_data: serde_json::to_string(&dynamic_template_data)?,
				..Default::default()
			},
		],
		attachments: Vec::new(),
		template_id: template_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "email_verification.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"verification_id": verification_id,
					"game_id": ctx.game_id.map(|x| x.as_uuid()),
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(email_verification::create::Response {
		verification_id: Some(verification_id.into()),
	})
}

const CODE_CHARSET: &[u8] = b"ABCDEFGHIJKLMNPQRSTUVWXYZ123456789";
const CODE_LEN: usize = 8;
fn gen_code() -> String {
	let mut rng = rand::thread_rng();
	std::iter::repeat_with(|| {
		let idx = rng.gen_range(0..CODE_CHARSET.len());
		CODE_CHARSET[idx] as char
	})
	.take(CODE_LEN)
	.collect::<String>()
}
