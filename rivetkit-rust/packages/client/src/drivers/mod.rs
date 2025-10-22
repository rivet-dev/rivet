use std::sync::Arc;

use crate::{
    protocol::{query, to_client, to_server},
    remote_manager::RemoteManager,
    EncodingKind, TransportKind
};
use anyhow::Result;
use serde_json::Value;
use tokio::{
    sync::mpsc,
    task::{AbortHandle, JoinHandle},
};
use tracing::debug;

pub mod sse;
pub mod ws;

pub type MessageToClient = Arc<to_client::ToClient>;
pub type MessageToServer = Arc<to_server::ToServer>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverStopReason {
    UserAborted,
    ServerDisconnect,
    ServerError,
    TaskError,
}

#[derive(Debug)]
pub struct DriverHandle {
    abort_handle: AbortHandle,
    sender: mpsc::Sender<MessageToServer>,
}

impl DriverHandle {
    pub fn new(sender: mpsc::Sender<MessageToServer>, abort_handle: AbortHandle) -> Self {
        Self {
            sender,
            abort_handle,
        }
    }

    pub async fn send(&self, msg: Arc<to_server::ToServer>) -> Result<()> {
        self.sender.send(msg).await?;

        Ok(())
    }

    pub fn disconnect(&self) {
        self.abort_handle.abort();
    }
}

impl Drop for DriverHandle {
    fn drop(&mut self) {
        debug!("DriverHandle dropped, aborting task");
        self.disconnect()
    }
}

pub type DriverConnection = (
    DriverHandle,
    mpsc::Receiver<MessageToClient>,
    JoinHandle<DriverStopReason>,
);

pub struct DriverConnectArgs {
    pub remote_manager: RemoteManager,
    pub encoding_kind: EncodingKind,
    pub query: query::ActorQuery,
    pub parameters: Option<Value>,
    pub conn_id: Option<String>,
    pub conn_token: Option<String>,
}

pub async fn connect_driver(
    transport_kind: TransportKind,
    args: DriverConnectArgs
) -> Result<DriverConnection> {
    let res = match transport_kind {
        TransportKind::WebSocket => ws::connect(args).await?,
        TransportKind::Sse => sse::connect(args).await?,
    };

    Ok(res)
}
