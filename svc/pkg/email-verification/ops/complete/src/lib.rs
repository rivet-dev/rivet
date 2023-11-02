use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

const MAX_ATTEMPTS: i64 = 4;

#[derive(sqlx::FromRow)]
struct VerificationRow {
	email: String,
	code: String,
	expire_ts: i64,
	complete_ts: Option<i64>,
}

impl VerificationRow {
	async fn build_response(
		self,
		client: &chirp_client::Client,
		verification_id: Uuid,
		status: email_verification::complete::response::Status,
	) -> GlobalResult<email_verification::complete::Response> {
		if status == email_verification::complete::response::Status::Correct {
			msg!([client] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
						name: "email_verification.complete".into(),
						properties_json: Some(serde_json::to_string(&json!({
							"verification_id": verification_id,
						}))?),
						..Default::default()
					}
				],
			})
			.await?;
		} else {
			msg!([client] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
						name: "email_verification.error".into(),
						properties_json: Some(serde_json::to_string(&json!({
							"verification_id": verification_id,
							"error": status as i32,
						}))?),
						..Default::default()
					}
				],
			})
			.await?;
		}

		Ok(email_verification::complete::Response {
			status: status as i32,
			email: self.email,
		})
	}
}

#[derive(sqlx::FromRow)]
struct CompleteLwtRow {
	applied: bool,
	_complete_ts: Option<chrono::Duration>,
}

#[operation(name = "email-verification-complete")]
async fn handle(
	ctx: OperationContext<email_verification::complete::Request>,
) -> GlobalResult<email_verification::complete::Response> {
	// TODO: Use a CRDB transaction

	let crdb = ctx.crdb().await?;

	let verification_id = unwrap_ref!(ctx.verification_id).as_uuid();

	// Fetch metadata
	let verification = sql_fetch_one!(
		[ctx, VerificationRow]
		"
		SELECT email, code, expire_ts, complete_ts
		FROM db_email_verification.verifications
		WHERE verification_id = $1
		",
		verification_id,
	)
	.await?;

	// Validate the code is not complete or expired in order to prevent spamming
	// attempt insertions.
	if verification.complete_ts.is_some() {
		return verification
			.build_response(
				ctx.chirp(),
				verification_id,
				email_verification::complete::response::Status::AlreadyComplete,
			)
			.await;
	} else if ctx.ts() > verification.expire_ts {
		return verification
			.build_response(
				ctx.chirp(),
				verification_id,
				email_verification::complete::response::Status::Expired,
			)
			.await;
	}

	// Log attempt
	let (attempt_count,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		WITH ins AS (
			INSERT INTO db_email_verification.verification_attempts (verification_id, attempt_id, create_ts)
			VALUES ($1, $2, $3)
			RETURNING 1
		)
		SELECT COUNT(*)
		FROM db_email_verification.verification_attempts
		WHERE verification_id = $1
		LIMIT $4
		",
		verification_id,
		Uuid::new_v4(),
		ctx.ts(),
		MAX_ATTEMPTS,
	)
	.await?;

	// Validate attempts
	if attempt_count >= MAX_ATTEMPTS {
		return verification
			.build_response(
				ctx.chirp(),
				verification_id,
				email_verification::complete::response::Status::TooManyAttempts,
			)
			.await;
	}

	// Validate code. Do this after logging the attempt in order to ensure
	// there's no brute force.
	if ctx.code != verification.code {
		return verification
			.build_response(
				ctx.chirp(),
				verification_id,
				email_verification::complete::response::Status::Incorrect,
			)
			.await;
	}

	// Complete verification
	let complete_res = sql_query!(
		[ctx]
		"
		UPDATE db_email_verification.verifications
		SET complete_ts = $2
		WHERE verification_id = $1 AND complete_ts IS NULL
		",
		verification_id,
		ctx.ts(),
	)
	.await?;
	if complete_res.rows_affected() > 0 {
		return verification
			.build_response(
				ctx.chirp(),
				verification_id,
				email_verification::complete::response::Status::Correct,
			)
			.await;
	} else {
		tracing::info!("verification complete in race condition");

		return verification
			.build_response(
				ctx.chirp(),
				verification_id,
				email_verification::complete::response::Status::AlreadyComplete,
			)
			.await;
	}
}
