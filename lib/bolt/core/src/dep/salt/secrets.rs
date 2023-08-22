use anyhow::*;
use serde_json::{json, Value};

use crate::context::{ProjectContext, S3Provider};

/// Generates a config that will be exposed to Salt.
pub async fn build_secrets(ctx: &ProjectContext) -> Result<Value> {
	let mut secrets = json!({});

	secrets["rivet"] = json!({
		"api_route": {
			"token": ctx.read_secret(&["rivet", "api_route", "token"]).await?,
		}
	});

	secrets["s3"] = s3(ctx).await?;

	secrets["clickhouse"] = json!({
		"users": {
			"bolt": {
				"password": ctx.read_secret(&["clickhouse", "users", "bolt", "password"]).await?,
			},
			"chirp": {
				"password": ctx.read_secret(&["clickhouse", "users", "chirp", "password"]).await?,
			},
			"grafana": {
				"password": ctx.read_secret(&["clickhouse", "users", "grafana", "password"]).await?,
			},
		},
	});

	if ctx.ns().s3.providers.minio.is_some() {
		secrets["minio"] = json!({
			"users": {
				"root": {
					"password": ctx.read_secret(&["minio", "users", "root", "password"]).await?,
				},
			},
		});
	}

	if let (Some(client_id), Some(client_secret)) = (
		ctx.read_secret_opt(&["cloudflare", "access", "proxy", "client_id"])
			.await?,
		ctx.read_secret_opt(&["cloudflare", "access", "proxy", "client_secret"])
			.await?,
	) {
		secrets["cloudflare"] = json!({
			"access": {
				"proxy": {
					"client_id": client_id,
					"client_secret": client_secret,
				},
			},
		});
	}

	Ok(secrets)
}

async fn s3(ctx: &ProjectContext) -> Result<Value> {
	let mut res = serde_json::Map::with_capacity(1);

	let (default_provider, _) = ctx.default_s3_provider()?;
	let default_s3_creds = ctx.s3_credentials(default_provider).await?;
	res.insert(
		"default".to_string(),
		json!({
			"persistent_access_key_id": default_s3_creds.access_key_id,
			"persistent_access_key_secret": default_s3_creds.access_key_secret,
		}),
	);

	let providers = &ctx.ns().s3.providers;
	if providers.minio.is_some() {
		let s3_creds = ctx.s3_credentials(S3Provider::Minio).await?;
		res.insert(
			"minio".to_string(),
			json!({
				"persistent_access_key_id": s3_creds.access_key_id,
				"persistent_access_key_secret": s3_creds.access_key_secret,
			}),
		);
	}
	if providers.backblaze.is_some() {
		let s3_creds = ctx.s3_credentials(S3Provider::Backblaze).await?;
		res.insert(
			"backblaze".to_string(),
			json!({
				"persistent_access_key_id": s3_creds.access_key_id,
				"persistent_access_key_secret": s3_creds.access_key_secret,
			}),
		);
	}
	if providers.aws.is_some() {
		let s3_creds = ctx.s3_credentials(S3Provider::Aws).await?;
		res.insert(
			"aws".to_string(),
			json!({
				"persistent_access_key_id": s3_creds.access_key_id,
				"persistent_access_key_secret": s3_creds.access_key_secret,
			}),
		);
	}

	Ok(res.into())
}
