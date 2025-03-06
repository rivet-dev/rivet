use rand::{distributions::Alphanumeric, Rng};

pub mod api;
pub mod client;
pub mod consts;

/// Generates a random string for a secret.
pub(crate) fn generate_password(length: usize) -> String {
	rand::thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length)
		.map(char::from)
		.collect()
}
