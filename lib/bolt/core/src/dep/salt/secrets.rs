use anyhow::*;
use serde_json::{json, Value};

use crate::context::ProjectContext;

/// Generates a config that will be exposed to Salt.
pub async fn build_secrets(ctx: &ProjectContext) -> Result<Value> {
	let mut secrets = json!({});

	secrets["rivet"] = json!({
		"api_route": {
			"token": ctx.read_secret(&["rivet", "api_route", "token"]).await?,
		}
	});

	let s3_creds = ctx.s3_credentials().await?;
	secrets["s3"] = json!({
		"persistent_access_key_id": s3_creds.access_key_id,
		"persistent_access_key_secret": s3_creds.access_key_secret,
	});

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

	secrets["minio"] = json!({
		"users": {
			"root": {
				"password": ctx.read_secret(&["minio", "users", "root", "password"]).await?,
			},
		},
	});

	Ok(secrets)
}
