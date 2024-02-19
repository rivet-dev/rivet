use jsonwebtoken::{EncodingKey, Header};
use prost::Message;
use proto::{backend::pkg::*, claims};
use rivet_claims::{ClaimsDecode, EntitlementTag};
use rivet_operation::prelude::*;

lazy_static::lazy_static! {
	/// The private EdDSA key in a PEM format. Corresponds to
	/// `rivet_claims::Config::jwt_key_public`.
	static ref JWT_KEY_PRIVATE: String = std::env::var("RIVET_JWT_KEY_PRIVATE").unwrap();
}

#[operation(name = "token-create")]
async fn handle(
	ctx: OperationContext<token::create::Request>,
) -> GlobalResult<token::create::Response> {
	ensure!(!ctx.issuer.is_empty());
	let token_config = unwrap_ref!(ctx.token_config);
	let kind = unwrap_ref!(ctx.kind);
	if ctx.combine_refresh_token {
		ensure!(
			ctx.refresh_token_config.is_none(),
			"cannot use combined refresh token with separate refresh token config"
		);
	}

	// Resolve required entitlements and current session ID. If
	// `current_refresh_token` is None and the request requires a refresh token,
	// a new one will be generated automatically. `current_refresh_token` may be
	// None when `session_id` is not if we decide we want to reissue the refresh
	// token.
	let (mut ent, session_id, current_refresh_token): (
		Vec<claims::Entitlement>,
		Option<Uuid>,
		Option<TokenData>,
	) = match kind {
		token::create::request::Kind::New(token::create::request::KindNew { entitlements }) => {
			(entitlements.clone(), None, None)
		}
		token::create::request::Kind::Refresh(token::create::request::KindRefresh {
			refresh_token,
		}) => {
			// TODO: There is a race condition here with revoking the token

			// Validate the token
			let refresh_token_claims = rivet_claims::decode(refresh_token)??;
			let refresh_jti = unwrap!(refresh_token_claims.jti).as_uuid();
			let refresh_ent = refresh_token_claims.as_refresh()?;

			// Check if revoked
			// TODO: This is a race condition, es no bueno. See note in token-revoke
			let rf_token_row = sql_fetch_optional!(
				[ctx, (Option<i64>,)]
				"SELECT revoke_ts FROM db_token.tokens WHERE jti = $1",
				refresh_jti,
			)
			.await?;
			if let Some((revoke_ts,)) = rf_token_row {
				if revoke_ts.is_some() {
					bail_with!(TOKEN_REVOKED);
				}
			} else {
				// TODO: The token may have expired here because of the TTL
				bail_with!(TOKEN_REFRESH_NOT_FOUND);
			}

			// Fetch entitlements to create the token with
			let session_row = sql_fetch_optional!(
				[ctx, (Vec<Vec<u8>>,)]
				"SELECT entitlements FROM db_token.sessions WHERE session_id = $1",
				refresh_ent.session_id,
			)
			.await?;
			let ent = unwrap!(session_row, "token session not found").0;

			// Decode entitlements
			let ent = ent
				.into_iter()
				.map(|ent_buf| claims::Entitlement::decode(ent_buf.as_slice()))
				.collect::<Result<Vec<_>, _>>()?;

			// Check if we should issue a new refresh token
			let issue_new_refresh = true; // TODO: Change this once we have unit tests to handle auto-refresh
			let current_refresh_token = if issue_new_refresh {
				// Revoke the refresh token
				let update_query = sql_execute!(
					[ctx]
					"
					UPDATE db_token.tokens
					SET revoke_ts = $1
					WHERE jti = $2 AND revoke_ts IS NULL AND exp > $1
					",
					ctx.ts(),
					refresh_jti,
				)
				.await?;

				if update_query.rows_affected() == 0 {
					tracing::info!("token revoked in race condition");
					bail_with!(TOKEN_REVOKED);
				}

				None
			} else {
				Some(TokenData {
					jti: refresh_jti,
					token: refresh_token.clone(),
				})
			};

			(ent, Some(refresh_ent.session_id), current_refresh_token)
		}
	};

	// Create session if needed
	let session_id = if let Some(session_id) = session_id {
		session_id
	} else {
		let new_session_id = Uuid::new_v4();

		// Add session to entitlements for the new tokens if we're using a combined
		// refresh & user token. When we refresh this token, the refresh
		// entitlement will automatically be part of the token so we don't need
		// a separate refresh token.
		if let (true, token::create::request::Kind::New(_)) = (ctx.combine_refresh_token, &kind) {
			ent.push(claims::Entitlement {
				kind: Some(claims::entitlement::Kind::Refresh(
					claims::entitlement::Refresh {
						session_id: Some(new_session_id.into()),
					},
				)),
			});
		}

		// Save session
		let ent_bufs = ent
			.iter()
			.map(|ent| -> GlobalResult<Vec<u8>> {
				let mut ent_buf = Vec::with_capacity(ent.encoded_len());
				ent.encode(&mut ent_buf)?;
				Ok(ent_buf)
			})
			.collect::<GlobalResult<Vec<Vec<u8>>>>()?;
		if !ctx.ephemeral {
			let tags = ent
				.iter()
				.flat_map(|x| x.tag().map(|x| x as i64))
				.collect::<Vec<_>>();
			sql_execute!(
				[ctx]
				"INSERT INTO db_token.sessions (session_id, entitlements, entitlement_tags, exp) VALUES ($1, $2, $3, $4)",
				new_session_id,
				ent_bufs,
				&tags,
				ctx.ts() + token_config.ttl,
			).await?;
		}

		new_session_id
	};

	// Resolve the refresh token
	let refresh_token = if let Some(token) = current_refresh_token {
		// Preserve existing refresh token
		Some(token)
	} else if let Some(refresh_token_config) = &ctx.refresh_token_config {
		// Create new refresh token
		let refresh_token = create_token(
			&ctx,
			if let Some(req_label) = &ctx.label {
				Some(format!("{}_rf", req_label))
			} else {
				Some("rf".to_owned())
			},
			refresh_token_config,
			Uuid::new_v4(),
			None,
			&[claims::Entitlement {
				kind: Some(claims::entitlement::Kind::Refresh(
					claims::entitlement::Refresh {
						session_id: Some(session_id.into()),
					},
				)),
			}],
			session_id,
			ctx.ephemeral,
		)
		.await?;

		Some(refresh_token)
	} else {
		// No refresh token is needed
		None
	};

	// Create token
	let jti = Uuid::new_v4();
	let token = create_token(
		&ctx,
		ctx.label.clone(),
		token_config,
		jti,
		refresh_token.as_ref().map(|x| x.jti),
		&ent,
		session_id,
		ctx.ephemeral,
	)
	.await?;

	Ok(token::create::Response {
		token: Some(token::create::response::TokenData {
			token: token.token,
			jti: Some(token.jti.into()),
		}),
		refresh_token: refresh_token.map(|t| token::create::response::TokenData {
			token: t.token,
			jti: Some(t.jti.into()),
		}),
		session_id: Some(session_id.into()),
	})
}

struct TokenData {
	jti: Uuid,
	token: String,
}

async fn create_token(
	ctx: &OperationContext<token::create::Request>,
	label: Option<impl AsRef<str>>,
	token_config: &token::create::request::TokenConfig,
	jti: Uuid,
	refresh_jti: Option<Uuid>,
	ent: &[claims::Entitlement],
	session_id: Uuid,
	ephemeral: bool,
) -> GlobalResult<TokenData> {
	// Create claims
	let exp = if token_config.ttl > 0 {
		Some(ctx.ts() + token_config.ttl)
	} else {
		None
	};
	let iat = ctx.ts();
	let claims = claims::Claims {
		jti: Some(jti.into()),
		exp,
		iat,
		entitlements: ent.to_vec(),
	};

	// Encode token
	tracing::info!(?claims, "encoding");
	let token = encode(
		label,
		&Header {
			alg: rivet_claims::ALGORITHM,
			..Default::default()
		},
		&claims,
		&EncodingKey::from_ed_pem(JWT_KEY_PRIVATE.as_bytes())?,
	)?;

	// Write to database
	let mut claims_buf = Vec::with_capacity(claims.encoded_len());
	claims.encode(&mut claims_buf)?;
	tracing::info!(buf_len = %claims_buf.len(), "writing claims");
	if !ephemeral {
		// Create the token and update the session expiration as needed
		sql_execute!(
			[ctx]
			"
			WITH
				_insert AS (
					INSERT INTO db_token.tokens (
						jti,
						exp,
						iat,
						refresh_jti,
						session_id,
						issuer,
						user_agent,
						remote_address
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
					RETURNING 1
				),
				_update_session AS (
					UPDATE db_token.sessions
					SET exp  = $2
					WHERE session_id = $5 AND exp < $2
					RETURNING 1
				)
			SELECT 1
			",
			jti,
			claims.exp,
			claims.iat,
			refresh_jti,
			session_id,
			&ctx.issuer,
			ctx.client.as_ref().map(|x| &x.user_agent),
			ctx.client.as_ref().map(|x| &x.remote_address),
		)
		.await?;
	}

	Ok(TokenData { jti, token })
}

/// `label` is an arbitrary string that can be provided in order to provide debug information about
/// what this token represents.
///
/// Modified from jsonwebtoken::encode to use Protobuf.
fn encode(
	label: Option<impl AsRef<str>>,
	header: &Header,
	claims: &claims::Claims,
	key: &EncodingKey,
) -> GlobalResult<String> {
	// TODO:
	// if key.family != header.alg.family() {
	//	 return Err(jsonwebtoken::errors::Error::from(
	//		 jsonwebtoken::errors::ErrorKind::InvalidAlgorithm,
	//	 ));
	// }

	// Serialize claims to be encoded
	let mut claims_buf = Vec::with_capacity(claims.encoded_len());
	claims.encode(&mut claims_buf)?;

	// Encode the claims
	let encoded_header =
		base64::encode_config(serde_json::to_string(&header)?, base64::URL_SAFE_NO_PAD);
	let encoded_claims = base64::encode_config(&claims_buf, base64::URL_SAFE_NO_PAD);
	let message = [encoded_header.as_ref(), encoded_claims.as_ref()].join(".");
	let signature = jsonwebtoken::crypto::sign(message.as_bytes(), key, header.alg)?;

	let token = [message, signature].join(".");
	if let Some(label) = label {
		Ok(format!("{}.{}", label.as_ref(), token))
	} else {
		Ok(token)
	}
}
