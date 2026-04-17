use std::{ops::Deref, sync::{Arc, Mutex}};
use serde_json::Value as JsonValue;
use anyhow::{anyhow, Result};
use crate::{
    common::{EncodingKind, TransportKind, HEADER_ENCODING, HEADER_CONN_PARAMS},
    connection::{start_connection, ActorConnection, ActorConnectionInner},
    protocol::{codec, query::*},
    remote_manager::RemoteManager,
};

pub use crate::protocol::codec::{QueueSendResult, QueueSendStatus};

#[derive(Default)]
pub struct QueueSendOptions {
    pub timeout: Option<u64>,
}

pub struct ActorHandleStateless {
    remote_manager: RemoteManager,
    params: Option<JsonValue>,
    encoding_kind: EncodingKind,
    // Mutex (not RefCell) so the handle is `Sync` and `&handle` futures
    // remain `Send` — required to call `.action(...)` from within axum
    // middleware that needs `Send` futures.
    query: Mutex<ActorQuery>,
}

impl ActorHandleStateless {
    pub fn new(
        remote_manager: RemoteManager,
        params: Option<JsonValue>,
        encoding_kind: EncodingKind,
        query: ActorQuery
    ) -> Self {
        Self {
            remote_manager,
            params,
            encoding_kind,
            query: Mutex::new(query)
        }
    }

    pub async fn action(&self, name: &str, args: Vec<JsonValue>) -> Result<JsonValue> {
        // Resolve actor ID
        let query = self.query.lock().expect("query lock poisoned").clone();
        let actor_id = self.remote_manager.resolve_actor_id(&query).await?;

        let body = codec::encode_http_action_request(self.encoding_kind, &args)?;

        // Build headers
        let mut headers = vec![
            (HEADER_ENCODING.to_string(), self.encoding_kind.to_string()),
        ];

        if let Some(params) = &self.params {
            headers.push((HEADER_CONN_PARAMS.to_string(), serde_json::to_string(params)?));
        }

        // Send request via gateway
        let path = format!("/action/{}", urlencoding::encode(name));
        let res = self.remote_manager.send_request(
            &actor_id,
            &path,
            "POST",
            headers,
            Some(body),
        ).await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.bytes().await?;
            if let Ok((group, code, message, metadata)) =
                codec::decode_http_error(self.encoding_kind, &body)
            {
                return Err(anyhow!(
                    "action failed ({group}/{code}): {message}, metadata={metadata:?}"
                ));
            }
            return Err(anyhow!("action failed: {status}"));
        }

        // Decode response
        let output = res.bytes().await?;
        codec::decode_http_action_response(self.encoding_kind, &output)
    }

    pub async fn send(&self, name: &str, body: JsonValue) -> Result<()> {
        self.send_queue(name, body, false, None).await.map(|_| ())
    }

    pub async fn send_and_wait(
        &self,
        name: &str,
        body: JsonValue,
        opts: QueueSendOptions,
    ) -> Result<QueueSendResult> {
        let result = self.send_queue(name, body, true, opts.timeout).await?;
        result.ok_or_else(|| anyhow!("queue wait response missing"))
    }

    async fn send_queue(
        &self,
        name: &str,
        body: JsonValue,
        wait: bool,
        timeout: Option<u64>,
    ) -> Result<Option<QueueSendResult>> {
        let query = self.query.lock().expect("query lock poisoned").clone();
        let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
        let request_body = codec::encode_http_queue_request(
            self.encoding_kind,
            name,
            &body,
            wait,
            timeout,
        )?;

        let mut headers = vec![
            (HEADER_ENCODING.to_string(), self.encoding_kind.to_string()),
        ];

        if let Some(params) = &self.params {
            headers.push((HEADER_CONN_PARAMS.to_string(), serde_json::to_string(params)?));
        }

        let path = format!("/queue/{}", urlencoding::encode(name));
        let res = self.remote_manager.send_request(
            &actor_id,
            &path,
            "POST",
            headers,
            Some(request_body),
        ).await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.bytes().await?;
            if let Ok((group, code, message, metadata)) =
                codec::decode_http_error(self.encoding_kind, &body)
            {
                return Err(anyhow!(
                    "queue send failed ({group}/{code}): {message}, metadata={metadata:?}"
                ));
            }
            return Err(anyhow!("queue send failed: {status}"));
        }

        let body = res.bytes().await?;
        let result = codec::decode_http_queue_response(self.encoding_kind, &body)?;
        Ok(wait.then_some(result))
    }

    pub async fn fetch(
        &self,
        path: &str,
        method: &str,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<reqwest::Response> {
        let query = self.query.lock().expect("query lock poisoned").clone();
        let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
        let path = normalize_fetch_path(path);
        self.remote_manager
            .send_request(&actor_id, &path, method, headers, body)
            .await
    }

    pub async fn web_socket(
        &self,
        path: &str,
        protocols: Vec<String>,
    ) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
        let query = self.query.lock().expect("query lock poisoned").clone();
        let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
        self.remote_manager
            .open_raw_websocket(&actor_id, path, self.params.clone(), protocols)
            .await
    }

    pub fn gateway_url(&self) -> Result<String> {
        let query = self.query.lock().expect("query lock poisoned").clone();
        self.remote_manager.gateway_url(&query)
    }

    pub fn get_gateway_url(&self) -> Result<String> {
        self.gateway_url()
    }

    pub async fn reload(&self) -> Result<()> {
        let query = self.query.lock().expect("query lock poisoned").clone();
        let actor_id = self.remote_manager.resolve_actor_id(&query).await?;
        let res = self.remote_manager.send_request(
            &actor_id,
            "/dynamic/reload",
            "PUT",
            Vec::new(),
            None,
        ).await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("reload failed with status {status}: {body}"));
        }
        Ok(())
    }

    pub async fn resolve(&self) -> Result<String> {
        let query = {
            let Ok(query) = self.query.lock() else {
                return Err(anyhow!("Failed to lock actor query"));
            };
            query.clone()
        };

        match query {
            ActorQuery::Create { .. } => {
                Err(anyhow!("actor query cannot be create"))
            },
            ActorQuery::GetForId { get_for_id } => {
                Ok(get_for_id.actor_id.clone())
            },
            _ => {
                let actor_id = self.remote_manager.resolve_actor_id(&query).await?;

                // Get name from the original query
                let name = match &query {
                    ActorQuery::GetForKey { get_for_key } => get_for_key.name.clone(),
                    ActorQuery::GetOrCreateForKey { get_or_create_for_key } => get_or_create_for_key.name.clone(),
                    _ => return Err(anyhow!("unexpected query type")),
                };

                {
                    let Ok(mut query_mut) = self.query.lock() else {
                        return Err(anyhow!("Failed to lock actor query mutably"));
                    };

                    *query_mut = ActorQuery::GetForId {
                        get_for_id: GetForIdRequest {
                            name,
                            actor_id: actor_id.clone(),
                        }
                    };
                }

                Ok(actor_id)
            }
        }
    }
}

fn normalize_fetch_path(path: &str) -> String {
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        "/request".to_string()
    } else {
        format!("/request/{path}")
    }
}

pub struct ActorHandle {
    handle: ActorHandleStateless,
    remote_manager: RemoteManager,
    params: Option<JsonValue>,
    query: ActorQuery,
    client_shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
    transport_kind: crate::TransportKind,
    encoding_kind: EncodingKind,
}

impl ActorHandle {
    pub fn new(
        remote_manager: RemoteManager,
        params: Option<JsonValue>,
        query: ActorQuery,
        client_shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
        transport_kind: TransportKind,
        encoding_kind: EncodingKind
    ) -> Self {
        let handle = ActorHandleStateless::new(
            remote_manager.clone(),
            params.clone(),
            encoding_kind,
            query.clone()
        );

        Self {
            handle,
            remote_manager,
            params,
            query,
            client_shutdown_tx,
            transport_kind,
            encoding_kind,
        }
    }

    pub fn connect(&self) -> ActorConnection {
        let conn = ActorConnectionInner::new(
            self.remote_manager.clone(),
            self.query.clone(),
            self.transport_kind,
            self.encoding_kind,
            self.params.clone()
        );

        let rx = self.client_shutdown_tx.subscribe();
        start_connection(&conn, rx);

        conn
    }
}

impl Deref for ActorHandle {
    type Target = ActorHandleStateless;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
