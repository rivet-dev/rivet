pub mod duration;
mod error;
pub mod extensions;
pub mod log_stream;
pub mod monitor;
mod util;

use nomad_client::apis::configuration::Configuration;

pub use crate::error::NomadError;

pub fn config_from_env() -> Result<Configuration, NomadError> {
	let nomad_addr = std::env::var("NOMAD_ADDRESS")
		.map_err(|_| NomadError::MissingEnvVar("NOMAD_ADDRESS".into()))?;
	let config = nomad_client::apis::configuration::Configuration {
		base_path: format!("{}/v1", nomad_addr),
		..Default::default()
	};

	Ok(config)
}
