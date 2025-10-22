use std::sync::Arc;

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


pub struct Client {
    remote_manager: RemoteManager,
    encoding_kind: EncodingKind,
    transport_kind: TransportKind,
    shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
}

impl Client {
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
}

impl Drop for Client {
    fn drop(&mut self) {
        // Notify all subscribers to shutdown
        let _ = self.shutdown_tx.send(());
    }
}