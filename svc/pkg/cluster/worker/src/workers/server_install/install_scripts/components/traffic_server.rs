use chirp_worker::prelude::*;
use include_dir::{include_dir, Dir};
use s3_util::Provider;

use super::s3;

const TRAFFIC_SERVER_IMAGE: &str = "ghcr.io/rivet-gg/apache-traffic-server:9934dc2";

pub fn install() -> String {
	include_str!("../files/traffic_server_install.sh").replace("__IMAGE__", TRAFFIC_SERVER_IMAGE)
}

pub async fn configure() -> GlobalResult<String> {
	// Write config to files
	let mut config_scripts = config()
		.await?
		.into_iter()
		.map(|(k, v)| format!("cat << 'EOF' > /etc/trafficserver/{k}\n{v}\nEOF\n"))
		.collect::<Vec<_>>();

	// Update default storage config size to be entire filesystem size minus 4GB
	config_scripts.push(
		indoc!(
			r#"
			df -h / |
			awk 'NR==2 {gsub(/G/, "", $2); print $2 - 4 "G"}' |
			xargs -I {} sed -i 's/64G/{}/' /etc/trafficserver/storage.config
			"#
		)
		.to_string(),
	);

	let script = include_str!("../files/traffic_server_configure.sh")
		.replace("__IMAGE__", TRAFFIC_SERVER_IMAGE)
		.replace("__CONFIG__", &config_scripts.join("\n\n"));

	Ok(script)
}

static TRAFFIC_SERVER_CONFIG_DIR: Dir<'_> = include_dir!(
	"$CARGO_MANIFEST_DIR/src/workers/server_install/install_scripts/files/traffic_server"
);

async fn config() -> GlobalResult<Vec<(String, String)>> {
	// Static files
	let mut config_files = Vec::new();
	collect_config_files(&TRAFFIC_SERVER_CONFIG_DIR, &mut config_files)?;

	// Storage (default value of 64 gets overwritten in config script)
	let volume_size = 64;
	config_files.push((
		"storage.config".to_string(),
		format!("/var/cache/trafficserver {volume_size}G"),
	));

	// Remap & S3
	let mut remap = String::new();
	let default_s3_provider = Provider::default()?;
	if s3_util::s3_provider_active("bucket-build", Provider::Minio) {
		let output = s3::gen_provider(Provider::Minio, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if s3_util::s3_provider_active("bucket-build", Provider::Backblaze) {
		let output = s3::gen_provider(Provider::Backblaze, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if s3_util::s3_provider_active("bucket-build", Provider::Aws) {
		let output = s3::gen_provider(Provider::Aws, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	config_files.push(("remap.config".to_string(), remap));

	Ok(config_files)
}

// Recursively collects all of the files in a folder into a hashmap
fn collect_config_files(
	dir: &include_dir::Dir,
	config_files: &mut Vec<(String, String)>,
) -> GlobalResult<()> {
	for entry in dir.entries() {
		match entry {
			include_dir::DirEntry::File(file) => {
				let key = unwrap!(unwrap!(file.path().file_name()).to_str()).to_string();

				let value = unwrap!(file.contents_utf8());
				config_files.push((key, value.to_string()));
			}
			include_dir::DirEntry::Dir(dir) => collect_config_files(dir, config_files)?,
		}
	}

	Ok(())
}
