use chirp_workflow::prelude::*;

pub async fn install() -> GlobalResult<String> {
	let s3_client =
		s3_util::Client::from_env_opt("bucket-infra-artifacts", s3_util::EndpointKind::External)
			.await?;

	let (manager_binary_url, container_runner_binary_url, v8_isolate_runner_url) = tokio::try_join!(
		resolve_binary_url(&s3_client, util::env::var("PEGBOARD_MANAGER_BINARY_KEY")?),
		resolve_binary_url(&s3_client, util::env::var("CONTAINER_RUNNER_BINARY_KEY")?),
		resolve_binary_url(&s3_client, util::env::var("V8_ISOLATE_RUNNER_BINARY_KEY")?),
	)?;

	Ok(include_str!("../files/pegboard_install.sh")
		.replace("__MANAGER_BINARY_URL__", &manager_binary_url)
		.replace(
			"__CONTAINER_RUNNER_BINARY_URL__",
			&container_runner_binary_url,
		)
		.replace("__V8_ISOLATE_BINARY_URL__", &v8_isolate_runner_url))
}

pub fn configure() -> GlobalResult<String> {
	Ok(include_str!("../files/pegboard_configure.sh")
		.replace("__ORIGIN_API__", util::env::origin_api())
		// HACK: Hardcoded to Linode
		.replace("__PUBLIC_IFACE__", "eth0")
		// HACK: Hardcoded to Linode
		.replace("__VLAN_IFACE__", "eth1")
		.replace(
			"__GG_VLAN_SUBNET__",
			&util::net::gg::vlan_ip_net().to_string(),
		)
		.replace(
			"__ATS_VLAN_SUBNET__",
			&util::net::ats::vlan_ip_net().to_string(),
		))
}

/// Generates a presigned S3 URL for binaries.
async fn resolve_binary_url(
	s3_client: &s3_util::Client,
	file_name: String,
) -> GlobalResult<String> {
	let presigned_req = s3_client
		.get_object()
		.bucket(s3_client.bucket())
		.key(file_name)
		.presigned(
			s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
				.expires_in(std::time::Duration::from_secs(15 * 60))
				.build()?,
		)
		.await?;

	Ok(presigned_req.uri().clone().to_string())
}
