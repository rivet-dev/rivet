use chirp_worker::prelude::*;
// use rivet_convert::ApiTryFrom;
use rivet_operation::prelude::proto::backend::pkg::nomad;
// use serde_json::Value;

pub async fn call(
	_ctx: &OperationContext<nomad::msg::monitor_alloc_update::Message>,
	_alloc_id: String,
) -> GlobalResult<()> {
	// let ctx = ctx.clone();
	// tokio::spawn(async move {
	// 	// Get the server from the database. If it has a webhook_url, send all
	// 	// of the info about the server to it
	// 	tracing::error!(?alloc_id, "Checking Alloc ID");
	//
	// 	let server_id = match sql_fetch_optional!(
	// 		[ctx, (Uuid,)]
	// 		"
	// 		SELECT server_id
	// 		FROM db_ds.server_nomad
	// 		WHERE nomad_alloc_id = $1
	// 		",
	// 		alloc_id,
	// 	)
	// 	.await
	// 	{
	// 		Ok(Some(row)) => row,
	// 		Err(err) => {
	// 			tracing::error!(?err, "Could not find server from Nomad alloc");
	// 			return;
	// 		}
	// 		_ => {
	// 			tracing::error!("Could not find server from Nomad alloc");
	// 			return;
	// 		}
	// 	}
	// 	.0;
	//
	// 	let server = match op!([ctx] ds_server_get {
	// 		server_ids: vec![server_id.into()],
	// 	})
	// 	.await
	// 	{
	// 		Ok(server_res) => match server_res.servers.first() {
	// 			Some(server) => server.to_owned(),
	// 			None => {
	// 				tracing::error!("Could not get server from database");
	// 				return;
	// 			}
	// 		},
	// 		Err(err) => {
	// 			tracing::error!(?err, "Could not get server from database");
	// 			return;
	// 		}
	// 	};
	//
	// 	let client = reqwest::Client::builder()
	// 		.timeout(std::time::Duration::from_secs(15))
	// 		.build()
	// 		.expect("Failed to build client");
	//
	// 	let webhook_url = match &server.webhook_url {
	// 		Some(url) => url,
	// 		None => {
	// 			return;
	// 		}
	// 	};
	//
	// 	// Example of a JSON payload
	// 	let payload = serde_json::json!({
	// 		"message": match rivet_api::models::ServersServer::api_try_from(server.clone()) {
	// 			Ok(server) => server,
	// 			Err(err) => {
	// 				tracing::error!(?err, "Could not convert server to API");
	// 				return;
	// 			}
	// 		},
	// 	});
	//
	// 	match client.post(webhook_url).json(&payload).send().await {
	// 		Ok(response) => tracing::info!(?response, "Sent webhook"),
	// 		Err(err) => tracing::warn!(?err, "Issue sending webhook"),
	// 	};
	// });

	Ok(())
}
