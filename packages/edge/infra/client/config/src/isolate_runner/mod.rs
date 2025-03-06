use std::{net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};

pub mod actor;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
	pub actors_path: PathBuf,
	pub manager_ws_addr: SocketAddr,

	pub foundationdb: crate::manager::FoundationDb,
}
