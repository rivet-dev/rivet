use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use serde_cbor;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::{
    common::{
        ActorKey, EncodingKind, USER_AGENT_VALUE,
        HEADER_RIVET_TARGET, HEADER_RIVET_ACTOR, HEADER_RIVET_TOKEN,
        WS_PROTOCOL_STANDARD, WS_PROTOCOL_TARGET, WS_PROTOCOL_ACTOR,
        WS_PROTOCOL_ENCODING, WS_PROTOCOL_CONN_PARAMS, WS_PROTOCOL_CONN_ID,
        WS_PROTOCOL_CONN_TOKEN, WS_PROTOCOL_TOKEN, PATH_CONNECT_WEBSOCKET,
    },
    protocol::query::ActorQuery,
};

#[derive(Clone)]
pub struct RemoteManager {
    endpoint: String,
    token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Actor {
    actor_id: String,
    name: String,
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorsListResponse {
    actors: Vec<Actor>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorsGetOrCreateRequest {
    name: String,
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<String>, // base64-encoded CBOR
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorsGetOrCreateResponse {
    actor: Actor,
    created: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorsCreateRequest {
    name: String,
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<String>, // base64-encoded CBOR
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorsCreateResponse {
    actor: Actor,
}

impl RemoteManager {
    pub fn new(endpoint: &str, token: Option<String>) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            token,
        }
    }

    pub async fn get_for_id(&self, name: &str, actor_id: &str) -> Result<Option<String>> {
        let url = format!("{}/actors?name={}&actor_ids={}", self.endpoint, urlencoding::encode(name), urlencoding::encode(actor_id));

        let client = reqwest::Client::new();
        let mut req = client.get(&url).header(USER_AGENT, USER_AGENT_VALUE);

        if let Some(token) = &self.token {
            req = req.header(HEADER_RIVET_TOKEN, token);
        }

        let res = req.send().await?;

        if !res.status().is_success() {
            return Err(anyhow!("failed to get actor: {}", res.status()));
        }

        let data: ActorsListResponse = res.json().await?;

        if let Some(actor) = data.actors.first() {
            if actor.name == name {
                Ok(Some(actor.actor_id.clone()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_with_key(&self, name: &str, key: &ActorKey) -> Result<Option<String>> {
        let key_str = serde_json::to_string(key)?;
        let url = format!("{}/actors?name={}&key={}", self.endpoint, urlencoding::encode(name), urlencoding::encode(&key_str));

        let client = reqwest::Client::new();
        let mut req = client.get(&url).header(USER_AGENT, USER_AGENT_VALUE);

        if let Some(token) = &self.token {
            req = req.header(HEADER_RIVET_TOKEN, token);
        }

        let res = req.send().await?;

        if !res.status().is_success() {
            if res.status() == 404 {
                return Ok(None);
            }
            return Err(anyhow!("failed to get actor by key: {}", res.status()));
        }

        let data: ActorsListResponse = res.json().await?;

        if let Some(actor) = data.actors.first() {
            Ok(Some(actor.actor_id.clone()))
        } else {
            Ok(None)
        }
    }

    pub async fn get_or_create_with_key(
        &self,
        name: &str,
        key: &ActorKey,
        input: Option<serde_json::Value>,
    ) -> Result<String> {
        let key_str = serde_json::to_string(key)?;

        let input_encoded = if let Some(inp) = input {
            let cbor = serde_cbor::to_vec(&inp)?;
            Some(general_purpose::STANDARD.encode(cbor))
        } else {
            None
        };

        let request_body = ActorsGetOrCreateRequest {
            name: name.to_string(),
            key: key_str,
            input: input_encoded,
        };

        let client = reqwest::Client::new();
        let mut req = client
            .put(format!("{}/actors", self.endpoint))
            .header(USER_AGENT, USER_AGENT_VALUE)
            .json(&request_body);

        if let Some(token) = &self.token {
            req = req.header(HEADER_RIVET_TOKEN, token);
        }

        let res = req.send().await?;

        if !res.status().is_success() {
            return Err(anyhow!("failed to get or create actor: {}", res.status()));
        }

        let data: ActorsGetOrCreateResponse = res.json().await?;
        Ok(data.actor.actor_id)
    }

    pub async fn create_actor(
        &self,
        name: &str,
        key: &ActorKey,
        input: Option<serde_json::Value>,
    ) -> Result<String> {
        let key_str = serde_json::to_string(key)?;

        let input_encoded = if let Some(inp) = input {
            let cbor = serde_cbor::to_vec(&inp)?;
            Some(general_purpose::STANDARD.encode(cbor))
        } else {
            None
        };

        let request_body = ActorsCreateRequest {
            name: name.to_string(),
            key: key_str,
            input: input_encoded,
        };

        let client = reqwest::Client::new();
        let mut req = client
            .post(format!("{}/actors", self.endpoint))
            .header(USER_AGENT, USER_AGENT_VALUE)
            .json(&request_body);

        if let Some(token) = &self.token {
            req = req.header(HEADER_RIVET_TOKEN, token);
        }

        let res = req.send().await?;

        if !res.status().is_success() {
            return Err(anyhow!("failed to create actor: {}", res.status()));
        }

        let data: ActorsCreateResponse = res.json().await?;
        Ok(data.actor.actor_id)
    }

    pub async fn resolve_actor_id(&self, query: &ActorQuery) -> Result<String> {
        match query {
            ActorQuery::GetForId { get_for_id } => {
                self.get_for_id(&get_for_id.name, &get_for_id.actor_id)
                    .await?
                    .ok_or_else(|| anyhow!("actor not found"))
            }
            ActorQuery::GetForKey { get_for_key } => {
                self.get_with_key(&get_for_key.name, &get_for_key.key)
                    .await?
                    .ok_or_else(|| anyhow!("actor not found"))
            }
            ActorQuery::GetOrCreateForKey { get_or_create_for_key } => {
                self.get_or_create_with_key(
                    &get_or_create_for_key.name,
                    &get_or_create_for_key.key,
                    get_or_create_for_key.input.clone(),
                )
                .await
            }
            ActorQuery::Create { create } => {
                self.create_actor(&create.name, &create.key, create.input.clone())
                    .await
            }
        }
    }

    pub async fn send_request(
        &self,
        actor_id: &str,
        path: &str,
        method: &str,
        headers: Vec<(&str, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.endpoint, path);

        let client = reqwest::Client::new();
        let mut req = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes())?,
                &url,
            )
            .header(USER_AGENT, USER_AGENT_VALUE)
            .header(HEADER_RIVET_TARGET, "actor")
            .header(HEADER_RIVET_ACTOR, actor_id);

        if let Some(token) = &self.token {
            req = req.header(HEADER_RIVET_TOKEN, token);
        }

        for (key, value) in headers {
            req = req.header(key, value);
        }

        if let Some(body_data) = body {
            req = req.body(body_data);
        }

        let res = req.send().await?;
        Ok(res)
    }

    pub async fn open_websocket(
        &self,
        actor_id: &str,
        encoding: EncodingKind,
        params: Option<serde_json::Value>,
        conn_id: Option<String>,
        conn_token: Option<String>,
    ) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
        use tokio_tungstenite::connect_async;

        // Build WebSocket URL
        let ws_url = if self.endpoint.starts_with("https://") {
            format!("wss://{}{}", &self.endpoint[8..], PATH_CONNECT_WEBSOCKET)
        } else if self.endpoint.starts_with("http://") {
            format!("ws://{}{}", &self.endpoint[7..], PATH_CONNECT_WEBSOCKET)
        } else {
            return Err(anyhow!("invalid endpoint URL"));
        };

        // Build protocols
        let mut protocols = vec![
            WS_PROTOCOL_STANDARD.to_string(),
            format!("{}actor", WS_PROTOCOL_TARGET),
            format!("{}{}", WS_PROTOCOL_ACTOR, actor_id),
            format!("{}{}", WS_PROTOCOL_ENCODING, encoding.as_str()),
        ];

        if let Some(token) = &self.token {
            protocols.push(format!("{}{}", WS_PROTOCOL_TOKEN, token));
        }

        if let Some(p) = params {
            let params_str = serde_json::to_string(&p)?;
            protocols.push(format!("{}{}", WS_PROTOCOL_CONN_PARAMS, urlencoding::encode(&params_str)));
        }

        if let Some(cid) = conn_id {
            protocols.push(format!("{}{}", WS_PROTOCOL_CONN_ID, cid));
        }

        if let Some(ct) = conn_token {
            protocols.push(format!("{}{}", WS_PROTOCOL_CONN_TOKEN, ct));
        }

        let mut request = ws_url.into_client_request()?;
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            protocols.join(", ").parse()?,
        );

        let (ws_stream, _) = connect_async(request).await?;
        Ok(ws_stream)
    }
}
