use std::sync::Arc;
use std::collections::HashMap;

use anyhow::Result;
use serde_json::{Value as JsonValue};

use crate::{
    common::{ActorKey, EncodingKind, TransportKind},
    handle::ActorHandle,
    protocol::query::*,
    remote_manager::RemoteManager,
};

#[derive(Default)]
pub struct GetWithIdOptions {
    pub params: Option<JsonValue>,
}

#[derive(Default)]
pub struct GetOptions {
    pub params: Option<JsonValue>,
}

#[derive(Default)]
pub struct GetOrCreateOptions {
    pub params: Option<JsonValue>,
    pub create_in_region: Option<String>,
    pub create_with_input: Option<JsonValue>,
}

#[derive(Default)]
pub struct CreateOptions {
    pub params: Option<JsonValue>,
    pub region: Option<String>,
    pub input: Option<JsonValue>,
}

pub struct ClientConfig {
    pub endpoint: String,
    pub token: Option<String>,
    pub namespace: String,
    pub pool_name: String,
    pub encoding: EncodingKind,
    pub transport: TransportKind,
    pub headers: HashMap<String, String>,
    pub max_input_size: usize,
    pub disable_metadata_lookup: bool,
}

impl ClientConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: None,
            namespace: "default".to_string(),
            pool_name: "default".to_string(),
            encoding: EncodingKind::Bare,
            transport: TransportKind::WebSocket,
            headers: HashMap::new(),
            max_input_size: 4 * 1024,
            disable_metadata_lookup: false,
        }
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn token_opt(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn pool_name(mut self, pool_name: impl Into<String>) -> Self {
        self.pool_name = pool_name.into();
        self
    }

    pub fn encoding(mut self, encoding: EncodingKind) -> Self {
        self.encoding = encoding;
        self
    }

    pub fn transport(mut self, transport: TransportKind) -> Self {
        self.transport = transport;
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn max_input_size(mut self, max_input_size: usize) -> Self {
        self.max_input_size = max_input_size;
        self
    }

    pub fn disable_metadata_lookup(mut self, disable: bool) -> Self {
        self.disable_metadata_lookup = disable;
        self
    }
}


pub struct Client {
    remote_manager: RemoteManager,
    encoding_kind: EncodingKind,
    transport_kind: TransportKind,
    shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
}

impl Client {
    pub fn from_config(config: ClientConfig) -> Self {
        let remote_manager = RemoteManager::from_config(
            config.endpoint,
            config.token,
            config.namespace,
            config.pool_name,
            config.headers,
            config.max_input_size,
            config.disable_metadata_lookup,
        );

        Self {
            remote_manager,
            encoding_kind: config.encoding,
            transport_kind: config.transport,
            shutdown_tx: Arc::new(tokio::sync::broadcast::channel(1).0)
        }
    }

    pub fn new(
        manager_endpoint: &str,
        transport_kind: TransportKind,
        encoding_kind: EncodingKind,
    ) -> Self {
        Self {
            remote_manager: RemoteManager::new(manager_endpoint, None),
            encoding_kind,
            transport_kind,
            shutdown_tx: Arc::new(tokio::sync::broadcast::channel(1).0)
        }
    }

    pub fn new_with_token(
        manager_endpoint: &str,
        token: String,
        transport_kind: TransportKind,
        encoding_kind: EncodingKind,
    ) -> Self {
        Self {
            remote_manager: RemoteManager::new(manager_endpoint, Some(token)),
            encoding_kind,
            transport_kind,
            shutdown_tx: Arc::new(tokio::sync::broadcast::channel(1).0)
        }
    }

    fn create_handle(
        &self,
        params: Option<JsonValue>,
        query: ActorQuery
    ) -> ActorHandle {
        let handle = ActorHandle::new(
            self.remote_manager.clone(),
            params,
            query,
            self.shutdown_tx.clone(),
            self.transport_kind,
            self.encoding_kind
        );

        handle
    }

    pub fn get(
        &self,
        name: &str,
        key: ActorKey,
        opts: GetOptions
    ) -> Result<ActorHandle> {
        let actor_query = ActorQuery::GetForKey {
            get_for_key: GetForKeyRequest {
                name: name.to_string(),
                key,
            }
        };

        let handle = self.create_handle(
            opts.params,
            actor_query
        );

        Ok(handle)
    }

    pub fn get_for_id(
        &self,
        name: &str,
        actor_id: &str,
        opts: GetOptions
    ) -> Result<ActorHandle> {
        let actor_query = ActorQuery::GetForId {
            get_for_id: GetForIdRequest {
                name: name.to_string(),
                actor_id: actor_id.to_string(),
            }
        };

        let handle = self.create_handle(
            opts.params,
            actor_query
        );

        Ok(handle)
    }

    pub fn get_or_create(
        &self,
        name: &str,
        key: ActorKey,
        opts: GetOrCreateOptions
    ) -> Result<ActorHandle> {
        let input = opts.create_with_input;
        let region = opts.create_in_region;

        let actor_query = ActorQuery::GetOrCreateForKey {
            get_or_create_for_key: GetOrCreateRequest {
                name: name.to_string(),
                key: key,
                input,
                region
            }
        };

        let handle = self.create_handle(
            opts.params,
            actor_query,
        );

        Ok(handle)
    }

    pub async fn create(
        &self,
        name: &str,
        key: ActorKey,
        opts: CreateOptions
    ) -> Result<ActorHandle> {
        let input = opts.input;
        let _region = opts.region;

        let actor_id = self.remote_manager.create_actor(
            name,
            &key,
            input,
        ).await?;

        let get_query = ActorQuery::GetForId {
            get_for_id: GetForIdRequest {
                name: name.to_string(),
                actor_id,
            }
        };

        let handle = self.create_handle(
            opts.params,
            get_query
        );

        Ok(handle)
    }

    pub fn disconnect(self) {
        drop(self)
    }

    pub fn dispose(self) {
        self.disconnect()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Notify all subscribers to shutdown
        let _ = self.shutdown_tx.send(());
    }
}
