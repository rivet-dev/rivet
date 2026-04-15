use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16};

use rivet_envoy_protocol as protocol;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::config::EnvoyConfig;
use crate::envoy::ToEnvoyMessage;

pub struct SharedContext {
	pub config: EnvoyConfig,
	pub envoy_key: String,
	pub envoy_tx: mpsc::UnboundedSender<ToEnvoyMessage>,
	pub ws_tx: Arc<Mutex<Option<mpsc::UnboundedSender<WsTxMessage>>>>,
	pub protocol_metadata: Arc<Mutex<Option<protocol::ProtocolMetadata>>>,
	pub protocol_version: AtomicU16,
	pub shutting_down: AtomicBool,
}

#[derive(Debug)]
pub enum WsTxMessage {
	Send(Vec<u8>),
	Close,
}
