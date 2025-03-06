use std::{net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};

pub mod actor;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
	pub actors_path: PathBuf,
	pub fdb_cluster_path: PathBuf,
	pub runner_addr: SocketAddr,
}
