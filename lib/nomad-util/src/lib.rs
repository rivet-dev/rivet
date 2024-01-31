pub mod duration;
mod error;
pub mod extensions;
pub mod log_stream;
pub mod monitor;
mod util;

use nomad_client::apis::configuration::Configuration;

pub use crate::error::NomadError;

pub fn config_from_env() -> Result<Configuration, NomadError> {
	let nomad_url =
		std::env::var("NOMAD_URL").map_err(|_| NomadError::MissingEnvVar("NOMAD_URL".into()))?;
	let config = nomad_client::apis::configuration::Configuration {
		base_path: format!("{}/v1", nomad_url),
		..Default::default()
	};

	Ok(config)
}
