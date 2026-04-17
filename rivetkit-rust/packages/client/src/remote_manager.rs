use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::header::{HeaderName, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_cbor;
use std::{collections::HashMap, str::FromStr};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::{
    common::{
        ActorKey, EncodingKind, USER_AGENT_VALUE,
        HEADER_RIVET_TARGET, HEADER_RIVET_ACTOR, HEADER_RIVET_TOKEN,
        HEADER_RIVET_NAMESPACE,
        WS_PROTOCOL_STANDARD, WS_PROTOCOL_TARGET, WS_PROTOCOL_ACTOR,
        WS_PROTOCOL_ENCODING, WS_PROTOCOL_CONN_PARAMS, WS_PROTOCOL_CONN_ID,
        WS_PROTOCOL_CONN_TOKEN, WS_PROTOCOL_TOKEN, PATH_CONNECT_WEBSOCKET,
        PATH_WEBSOCKET_PREFIX,
    },
    protocol::query::ActorQuery,
};

#[derive(Clone)]
pub struct RemoteManager {
    endpoint: String,
    token: Option<String>,
    namespace: String,
    pool_name: String,
    headers: HashMap<String, String>,
    max_input_size: usize,
    _disable_metadata_lookup: bool,
    client: reqwest::Client,
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
            namespace: "default".to_string(),
            pool_name: "default".to_string(),
            headers: HashMap::new(),
            max_input_size: 4 * 1024,
            _disable_metadata_lookup: false,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_config(
        endpoint: String,
        token: Option<String>,
        namespace: String,
        pool_name: String,
        headers: HashMap<String, String>,
        max_input_size: usize,
        disable_metadata_lookup: bool,
    ) -> Self {
        Self {
            endpoint,
            token,
            namespace,
            pool_name,
            headers,
            max_input_size,
            _disable_metadata_lookup: disable_metadata_lookup,
            client: reqwest::Client::new(),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    fn apply_common_headers(&self, mut req: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> {
        req = req.header(USER_AGENT, USER_AGENT_VALUE);

        for (key, value) in &self.headers {
            let name = HeaderName::from_str(key)
                .with_context(|| format!("invalid configured header name `{key}`"))?;
            let value = HeaderValue::from_str(value)
                .with_context(|| format!("invalid configured header value for `{key}`"))?;
            req = req.header(name, value);
        }

        if let Some(token) = &self.token {
            req = req.header(HEADER_RIVET_TOKEN, token);
        }

        if !self.namespace.is_empty() {
            req = req.header(HEADER_RIVET_NAMESPACE, &self.namespace);
        }

        Ok(req)
    }

    pub async fn get_for_id(&self, name: &str, actor_id: &str) -> Result<Option<String>> {
        let url = format!("{}/actors?name={}&actor_ids={}", self.endpoint, urlencoding::encode(name), urlencoding::encode(actor_id));

        let req = self.apply_common_headers(self.client.get(&url))?;

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

        let req = self.apply_common_headers(self.client.get(&url))?;

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

        let req = self.apply_common_headers(
            self.client
                .put(format!("{}/actors", self.endpoint))
                .json(&request_body),
        )?;

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

        let req = self.apply_common_headers(
            self.client
                .post(format!("{}/actors", self.endpoint))
                .json(&request_body),
        )?;

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
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<reqwest::Response> {
        let url = self.build_actor_gateway_url(actor_id, path);

        let mut req = self.apply_common_headers(self
            .client
            .request(
                reqwest::Method::from_bytes(method.as_bytes())?,
                &url,
            )
            .header(HEADER_RIVET_TARGET, "actor")
            .header(HEADER_RIVET_ACTOR, actor_id))?;

        for (key, value) in headers {
            req = req.header(key, value);
        }

        if let Some(body_data) = body {
            req = req.body(body_data);
        }

        let res = req.send().await?;
        Ok(res)
    }

    pub fn gateway_url(&self, query: &ActorQuery) -> Result<String> {
        match query {
            ActorQuery::GetForId { get_for_id } => {
                Ok(self.build_actor_gateway_url(&get_for_id.actor_id, ""))
            }
            ActorQuery::GetForKey { get_for_key } => {
                self.build_actor_query_gateway_url(
                    &get_for_key.name,
                    "get",
                    Some(&get_for_key.key),
                    None,
                    None,
                )
            }
            ActorQuery::GetOrCreateForKey { get_or_create_for_key } => {
                self.build_actor_query_gateway_url(
                    &get_or_create_for_key.name,
                    "getOrCreate",
                    Some(&get_or_create_for_key.key),
                    get_or_create_for_key.input.as_ref(),
                    get_or_create_for_key.region.as_deref(),
                )
            }
            ActorQuery::Create { .. } => {
                Err(anyhow!("gateway URL does not support create actor queries"))
            }
        }
    }

    pub fn build_actor_gateway_url(&self, actor_id: &str, path: &str) -> String {
        let token_segment = self
            .token
            .as_ref()
            .map(|token| format!("@{}", urlencoding::encode(token)))
            .unwrap_or_default();
        let gateway_path = format!(
            "/gateway/{}{}{}",
            urlencoding::encode(actor_id),
            token_segment,
            path,
        );
        combine_url_path(&self.endpoint, &gateway_path)
    }

    fn build_actor_query_gateway_url(
        &self,
        name: &str,
        method: &str,
        key: Option<&ActorKey>,
        input: Option<&serde_json::Value>,
        region: Option<&str>,
    ) -> Result<String> {
        if self.namespace.is_empty() {
            return Err(anyhow!("actor query namespace must not be empty"));
        }
        let mut params = Vec::new();
        push_query_param(&mut params, "rvt-namespace", &self.namespace);
        push_query_param(&mut params, "rvt-method", method);
        if let Some(key) = key {
            if !key.is_empty() {
                push_query_param(&mut params, "rvt-key", &key.join(","));
            }
        }
        if let Some(input) = input {
            let encoded = serde_cbor::to_vec(input)?;
            if encoded.len() > self.max_input_size {
                return Err(anyhow!(
                    "actor query input exceeds max_input_size ({} > {} bytes)",
                    encoded.len(),
                    self.max_input_size
                ));
            }
            push_query_param(&mut params, "rvt-input", &URL_SAFE_NO_PAD.encode(encoded));
        }
        if method == "getOrCreate" {
            push_query_param(&mut params, "rvt-runner", &self.pool_name);
            push_query_param(&mut params, "rvt-crash-policy", "sleep");
        }
        if let Some(region) = region {
            push_query_param(&mut params, "rvt-region", region);
        }
        if let Some(token) = &self.token {
            push_query_param(&mut params, "rvt-token", token);
        }

        let query = params.join("&");
        let path = format!("/gateway/{}?{}", urlencoding::encode(name), query);
        Ok(combine_url_path(&self.endpoint, &path))
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

        let ws_url = self.websocket_url(&self.build_actor_gateway_url(actor_id, PATH_CONNECT_WEBSOCKET))?;

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
        self.apply_websocket_headers(request.headers_mut())?;

        let (ws_stream, _) = connect_async(request).await?;
        Ok(ws_stream)
    }

    pub async fn open_raw_websocket(
        &self,
        actor_id: &str,
        path: &str,
        params: Option<serde_json::Value>,
        protocols: Vec<String>,
    ) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
        use tokio_tungstenite::connect_async;

        let gateway_path = normalize_raw_websocket_path(path);
        let ws_url = self.websocket_url(&self.build_actor_gateway_url(actor_id, &gateway_path))?;

        let mut all_protocols = vec![
            WS_PROTOCOL_STANDARD.to_string(),
            format!("{}actor", WS_PROTOCOL_TARGET),
            format!("{}{}", WS_PROTOCOL_ACTOR, actor_id),
        ];
        if let Some(token) = &self.token {
            all_protocols.push(format!("{}{}", WS_PROTOCOL_TOKEN, token));
        }
        if let Some(p) = params {
            let params_str = serde_json::to_string(&p)?;
            all_protocols.push(format!("{}{}", WS_PROTOCOL_CONN_PARAMS, urlencoding::encode(&params_str)));
        }
        all_protocols.extend(protocols);

        let mut request = ws_url.into_client_request()?;
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            all_protocols.join(", ").parse()?,
        );
        self.apply_websocket_headers(request.headers_mut())?;

        let (ws_stream, _) = connect_async(request).await?;
        Ok(ws_stream)
    }

    fn websocket_url(&self, url: &str) -> Result<String> {
        if let Some(rest) = url.strip_prefix("https://") {
            Ok(format!("wss://{rest}"))
        } else if let Some(rest) = url.strip_prefix("http://") {
            Ok(format!("ws://{rest}"))
        } else {
            Err(anyhow!("invalid endpoint URL"))
        }
    }

    fn apply_websocket_headers(&self, headers: &mut tokio_tungstenite::tungstenite::http::HeaderMap) -> Result<()> {
        for (key, value) in &self.headers {
            headers.insert(
                HeaderName::from_str(key)
                    .with_context(|| format!("invalid configured header name `{key}`"))?,
                HeaderValue::from_str(value)
                    .with_context(|| format!("invalid configured header value for `{key}`"))?,
            );
        }
        Ok(())
    }
}

fn combine_url_path(endpoint: &str, path: &str) -> String {
    format!("{}{}", endpoint.trim_end_matches('/'), path)
}

fn push_query_param(params: &mut Vec<String>, key: &str, value: &str) {
    params.push(format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)));
}

fn normalize_raw_websocket_path(path: &str) -> String {
    let mut path_portion = path;
    let mut query_portion = "";
    if let Some((left, right)) = path.split_once('?') {
        path_portion = left;
        query_portion = right;
    }
    let path_portion = path_portion.trim_start_matches('/');
    if query_portion.is_empty() {
        format!("{PATH_WEBSOCKET_PREFIX}{path_portion}")
    } else {
        format!("{PATH_WEBSOCKET_PREFIX}{path_portion}?{query_portion}")
    }
}
