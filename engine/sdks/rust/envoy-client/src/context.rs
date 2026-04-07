use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::config::EnvoyConfig;
use crate::envoy::ToEnvoyMessage;

pub struct SharedContext {
	pub config: EnvoyConfig,
	pub envoy_key: String,
	pub envoy_tx: mpsc::UnboundedSender<ToEnvoyMessage>,
	pub ws_tx: Arc<Mutex<Option<mpsc::UnboundedSender<WsTxMessage>>>>,
	pub protocol_metadata: Arc<Mutex<Option<protocol::ProtocolMetadata>>>,
	pub shutting_down: AtomicBool,
}

#[derive(Debug)]
pub enum WsTxMessage {
	Send(Vec<u8>),
	Close,
}
