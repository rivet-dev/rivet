pub mod duration;
mod error;
pub mod extensions;
pub mod log_stream;
pub mod monitor;
mod util;

use nomad_client::apis::configuration::Configuration;

pub use crate::error::NomadError;

pub fn build_config(config: &rivet_config::Config) -> Result<Configuration, NomadError> {
	let nomad_url = &config
		.server()
		.map_err(NomadError::Global)?
		.nomad()
		.map_err(NomadError::Global)?
		.url
		.to_string();
	let config = Configuration {
		base_path: format!("{}/v1", nomad_url),
		..Default::default()
	};

	Ok(config)
}

pub fn new_build_config(
	config: &rivet_config::Config,
) -> Result<nomad_client_new::apis::configuration::Configuration, NomadError> {
	let nomad_url = &config
		.server()
		.map_err(NomadError::Global)?
		.nomad()
		.map_err(NomadError::Global)?
		.url
		.to_string();
	let config = nomad_client_new::apis::configuration::Configuration {
		base_path: format!("{}/v1", nomad_url),
		..Default::default()
	};

	Ok(config)
}
