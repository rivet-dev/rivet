use std::{cell::RefCell, ops::Deref, sync::Arc};
use serde_json::Value as JsonValue;
use anyhow::{anyhow, Result};
use serde_cbor;
use crate::{
    common::{EncodingKind, TransportKind, HEADER_ENCODING, HEADER_CONN_PARAMS},
    connection::{start_connection, ActorConnection, ActorConnectionInner},
    protocol::query::*,
    remote_manager::RemoteManager,
};

pub struct ActorHandleStateless {
    remote_manager: RemoteManager,
    params: Option<JsonValue>,
    encoding_kind: EncodingKind,
    query: RefCell<ActorQuery>,
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
            query: RefCell::new(query)
        }
    }

    pub async fn action(&self, name: &str, args: Vec<JsonValue>) -> Result<JsonValue> {
        // Resolve actor ID
        let query = self.query.borrow().clone();
        let actor_id = self.remote_manager.resolve_actor_id(&query).await?;

        // Encode args as CBOR
        let args_cbor = serde_cbor::to_vec(&args)?;

        // Build headers
        let mut headers = vec![
            (HEADER_ENCODING, self.encoding_kind.to_string()),
        ];

        if let Some(params) = &self.params {
            headers.push((HEADER_CONN_PARAMS, serde_json::to_string(params)?));
        }

        // Send request via gateway
        let path = format!("/action/{}", urlencoding::encode(name));
        let res = self.remote_manager.send_request(
            &actor_id,
            &path,
            "POST",
            headers,
            Some(args_cbor),
        ).await?;

        if !res.status().is_success() {
            return Err(anyhow!("action failed: {}", res.status()));
        }

        // Decode response
        let output_cbor = res.bytes().await?;
        let output: JsonValue = serde_cbor::from_slice(&output_cbor)?;

        Ok(output)
    }

    pub async fn resolve(&self) -> Result<String> {
        let query = {
            let Ok(query) = self.query.try_borrow() else {
                return Err(anyhow!("Failed to borrow actor query"));
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
                    let Ok(mut query_mut) = self.query.try_borrow_mut() else {
                        return Err(anyhow!("Failed to borrow actor query mutably"));
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