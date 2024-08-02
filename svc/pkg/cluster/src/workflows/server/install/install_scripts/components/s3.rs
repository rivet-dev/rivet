use chirp_workflow::prelude::*;
use indoc::formatdoc;
use s3_util::Provider;

pub struct GenRemapS3ProviderOutput {
	/// Append to remap.config
	pub append_remap: String,

	/// Concat with config files
	pub config_files: Vec<(String, String)>,
}

pub async fn gen_provider(
	provider: Provider,
	default_s3_provider: Provider,
) -> GlobalResult<GenRemapS3ProviderOutput> {
	let mut remap = String::new();
	let provider_name = provider.as_str();
	let endpoint_external = s3_util::s3_endpoint_external("bucket-build", provider)?;
	let region = s3_util::s3_region("bucket-build", provider)?;
	let (access_key_id, secret_access_key) = s3_util::s3_credentials("bucket-build", provider)?;

	// Build plugin chain
	let plugins = format!("@plugin=tslua.so @pparam=/etc/trafficserver/strip_headers.lua @plugin=s3_auth.so @pparam=--config @pparam=s3_auth_v4_{provider_name}.config");

	// Add remap
	remap.push_str(&format!(
		"map /s3-cache/{provider_name} {endpoint_external} {plugins}\n",
	));

	// Add default route
	if default_s3_provider == provider {
		remap.push_str(&format!("map /s3-cache {endpoint_external} {plugins}\n",));
	}

	// Add credentials
	let mut config_files = Vec::<(String, String)>::new();
	config_files.push((
		format!("s3_auth_v4_{provider_name}.config"),
		formatdoc!(
			r#"
			access_key={access_key_id}
			secret_key={secret_access_key}
			version=4
			v4-region-map=s3_region_map_{provider_name}.config
			"#,
		),
	));
	config_files.push((
		format!("s3_region_map_{provider_name}.config"),
		formatdoc!(
			r#"
			# Default region
			{s3_host}: {s3_region}
			"#,
			s3_host = endpoint_external.split_once("://").unwrap().1,
			s3_region = region,
		),
	));

	Ok(GenRemapS3ProviderOutput {
		append_remap: remap,
		config_files,
	})
}
