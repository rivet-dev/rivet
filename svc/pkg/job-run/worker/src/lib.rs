pub mod workers;

lazy_static::lazy_static! {
	pub static ref NEW_NOMAD_CONFIG: nomad_client_new::apis::configuration::Configuration =
		nomad_util::new_config_from_env().unwrap();
}
