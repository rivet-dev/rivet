use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;

use crate::{
    protocol::to_server,
    protocol::to_client,
    EncodingKind
};

use super::{
    DriverConnectArgs, DriverConnection, DriverHandle, DriverStopReason, MessageToClient, MessageToServer
};

pub(crate) async fn connect(args: DriverConnectArgs) -> Result<DriverConnection> {
    // Resolve actor ID
    let actor_id = args.remote_manager.resolve_actor_id(&args.query).await?;

    debug!("Opening WebSocket connection to actor via gateway: {}", actor_id);

    // Open WebSocket via remote manager (gateway)
    let ws = args.remote_manager.open_websocket(
        &actor_id,
        args.encoding_kind,
        args.parameters,
        args.conn_id,
        args.conn_token,
    ).await.context("Failed to connect to WebSocket via gateway")?;

    let (in_tx, in_rx) = mpsc::channel::<MessageToClient>(32);
    let (out_tx, out_rx) = mpsc::channel::<MessageToServer>(32);

    let task = tokio::spawn(start(ws, args.encoding_kind, in_tx, out_rx));
    let handle = DriverHandle::new(out_tx, task.abort_handle());

    Ok((handle, in_rx, task))
}

async fn start(
    ws: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    encoding_kind: EncodingKind,
    in_tx: mpsc::Sender<MessageToClient>,
    mut out_rx: mpsc::Receiver<MessageToServer>,
) -> DriverStopReason {
    let (mut ws_sink, mut ws_stream) = ws.split();

    let serialize = get_msg_serializer(encoding_kind);
    let deserialize = get_msg_deserializer(encoding_kind);

    loop {
        tokio::select! {
            // Dispatch ws outgoing queue
            msg = out_rx.recv() => {
                // If the sender is dropped, break the loop
                let Some(msg) = msg else {
                    debug!("Sender dropped");
                    return DriverStopReason::UserAborted;
                };

                let msg = match serialize(&msg) {
                    Ok(msg) => msg,
                    Err(e) => {
                        debug!("Failed to serialize message: {:?}", e);
                        continue;
                    }
                };

                if let Err(e) = ws_sink.send(msg).await {
                    debug!("Failed to send message: {:?}", e);
                    continue;
                }
            },
            // Handle ws incoming
            msg = ws_stream.next() => {
                let Some(msg) = msg else {
                    println!("Receiver dropped");
                    return DriverStopReason::ServerDisconnect;
                };

                match msg {
                    Ok(msg) => match msg {
                        Message::Text(_) | Message::Binary(_) => {
                            let Ok(msg) = deserialize(&msg) else {
                                debug!("Failed to parse message: {:?}", msg);
                                continue;
                            };

                            if let Err(e) = in_tx.send(Arc::new(msg)).await {
                                debug!("Failed to send text message: {}", e);
                                // failure to send means user dropped incoming receiver
                                return DriverStopReason::UserAborted;
                            }
                        },
                        Message::Close(_) => {
                            debug!("Close message");
                            return DriverStopReason::ServerDisconnect;
                        },
                        _ => {
                            debug!("Invalid message type received");
                        }
                    }
                    Err(e) => {
                        debug!("WebSocket error: {}", e);
                        return DriverStopReason::ServerError;
                    }
                }
            }
        }
    }
}

fn get_msg_deserializer(encoding_kind: EncodingKind) -> fn(&Message) -> Result<to_client::ToClient> {
    match encoding_kind {
        EncodingKind::Json => json_msg_deserialize,
        EncodingKind::Cbor => cbor_msg_deserialize,
    }
}

fn get_msg_serializer(encoding_kind: EncodingKind) -> fn(&to_server::ToServer) -> Result<Message> {
    match encoding_kind {
        EncodingKind::Json => json_msg_serialize,
        EncodingKind::Cbor => cbor_msg_serialize,
    }
}

fn json_msg_deserialize(value: &Message) -> Result<to_client::ToClient> {
    match value {
        Message::Text(text) => Ok(serde_json::from_str(text)?),
        Message::Binary(bin) => Ok(serde_json::from_slice(bin)?),
        _ => Err(anyhow::anyhow!("Invalid message type")),
    }
}

fn cbor_msg_deserialize(value: &Message) -> Result<to_client::ToClient> {
    match value {
        Message::Binary(bin) => Ok(serde_cbor::from_slice(bin)?),
        Message::Text(text) => Ok(serde_cbor::from_slice(text.as_bytes())?),
        _ => Err(anyhow::anyhow!("Invalid message type")),
    }
}

fn json_msg_serialize(value: &to_server::ToServer) -> Result<Message> {
    Ok(Message::Text(serde_json::to_string(value)?.into()))
}

fn cbor_msg_serialize(value: &to_server::ToServer) -> Result<Message> {
    Ok(Message::Binary(serde_cbor::to_vec(value)?.into()))
}
