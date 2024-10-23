use chirp_workflow::prelude::*;
use indoc::formatdoc;

pub struct GenRemapS3Output {
	/// Append to remap.config
	pub append_remap: String,

	/// Concat with config files
	pub config_files: Vec<(String, String)>,
}

pub async fn gen_remap(config: &rivet_config::Config) -> GlobalResult<GenRemapS3Output> {
	let s3_config = &config.server()?.s3;

	let mut remap = String::new();
	let endpoint_external = s3_config.endpoint_external.to_string();

	// Build plugin chain
	let plugins = format!("@plugin=tslua.so @pparam=/etc/trafficserver/strip_headers.lua @plugin=s3_auth.so @pparam=--config @pparam=s3_auth_v4.config");

	// Add remap
	remap.push_str(&format!("map /s3-cache {endpoint_external} {plugins}\n",));

	// Add credentials
	let mut config_files = Vec::<(String, String)>::new();
	config_files.push((
		format!("s3_auth_v4.config"),
		formatdoc!(
			r#"
			access_key={access_key_id}
			secret_key={secret_access_key}
			version=4
			v4-region-map=s3_region_map.config
			"#,
			access_key_id = s3_config.access_key_id.read(),
			secret_access_key = s3_config.secret_access_key.read(),
		),
	));
	config_files.push((
		format!("s3_region_map.config"),
		formatdoc!(
			r#"
			# Default region
			{s3_host}: {s3_region}
			"#,
			s3_host = endpoint_external.split_once("://").unwrap().1,
			s3_region = s3_config.region,
		),
	));

	Ok(GenRemapS3Output {
		append_remap: remap,
		config_files,
	})
}
