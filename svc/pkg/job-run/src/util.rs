pub const NOMAD_REGION: &str = "global";

// Have to patch `nomad_client::apis::allocations_api::signal_allocation` because it uses `/allocation`
// instead of `/client/allocation`
pub async fn signal_allocation(
	configuration: &nomad_client::apis::configuration::Configuration,
	alloc_id: &str,
	namespace: Option<&str>,
	region: Option<&str>,
	index: Option<i64>,
	wait: Option<&str>,
	alloc_signal_request: Option<nomad_client::models::AllocSignalRequest>,
) -> Result<(), nomad_client::apis::Error<nomad_client::apis::allocations_api::SignalAllocationError>>
{
	let local_var_client = &configuration.client;

	let local_var_uri_str = format!(
		"{}/client/allocation/{alloc_id}/signal",
		configuration.base_path,
		alloc_id = nomad_client::apis::urlencode(alloc_id),
	);
	let mut local_var_req_builder = local_var_client.post(local_var_uri_str.as_str());

	if let Some(ref local_var_str) = namespace {
		local_var_req_builder =
			local_var_req_builder.query(&[("namespace", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_str) = region {
		local_var_req_builder =
			local_var_req_builder.query(&[("region", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_str) = index {
		local_var_req_builder =
			local_var_req_builder.query(&[("index", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_str) = wait {
		local_var_req_builder =
			local_var_req_builder.query(&[("wait", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_user_agent) = configuration.user_agent {
		local_var_req_builder =
			local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
	}
	local_var_req_builder = local_var_req_builder.json(&alloc_signal_request);

	let local_var_req = local_var_req_builder.build()?;
	let local_var_resp = local_var_client.execute(local_var_req).await?;

	let local_var_status = local_var_resp.status();
	let local_var_content = local_var_resp.text().await?;

	if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
		Ok(())
	} else {
		let local_var_entity: Option<nomad_client::apis::allocations_api::SignalAllocationError> =
			serde_json::from_str(&local_var_content).ok();
		let local_var_error = nomad_client::apis::ResponseContent {
			status: local_var_status,
			content: local_var_content,
			entity: local_var_entity,
		};
		Err(nomad_client::apis::Error::ResponseError(local_var_error))
	}
}
