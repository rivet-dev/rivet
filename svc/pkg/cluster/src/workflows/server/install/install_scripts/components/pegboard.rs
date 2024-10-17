use chirp_workflow::prelude::*;

pub async fn install() -> GlobalResult<String> {
	let binary_url = resolve_manager_binary_url().await?;

	Ok(include_str!("../files/pegboard_install.sh").replace("__BINARY_URL__", &binary_url))
}

pub fn configure() -> GlobalResult<String> {
	Ok(include_str!("../files/pegboard_configure.sh")
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

/// Generates a presigned URL for the pegboard manager binary.
async fn resolve_manager_binary_url() -> GlobalResult<String> {
	let file_name = util::env::var("PEGBOARD_MANAGER_BINARY_KEY")?;

	// Build client
	let s3_client =
		s3_util::Client::from_env_opt("bucket-infra-artifacts", s3_util::EndpointKind::External)
			.await?;
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

	let addr = presigned_req.uri().clone();

	let addr_str = addr.to_string();

	Ok(addr_str)
}
