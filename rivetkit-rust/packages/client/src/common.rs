
#[allow(dead_code)]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const USER_AGENT_VALUE: &str = concat!("ActorClient-Rust/", env!("CARGO_PKG_VERSION"));

// Headers
#[allow(dead_code)]
pub const HEADER_ACTOR_QUERY: &str = "x-rivet-query";
pub const HEADER_ENCODING: &str = "x-rivet-encoding";
pub const HEADER_CONN_PARAMS: &str = "x-rivet-conn-params";
#[allow(dead_code)]
pub const HEADER_ACTOR_ID: &str = "x-rivet-actor";
#[allow(dead_code)]
pub const HEADER_CONN_ID: &str = "x-rivet-conn";
#[allow(dead_code)]
pub const HEADER_CONN_TOKEN: &str = "x-rivet-conn-token";

// Gateway headers
pub const HEADER_RIVET_TARGET: &str = "x-rivet-target";
pub const HEADER_RIVET_ACTOR: &str = "x-rivet-actor";
pub const HEADER_RIVET_TOKEN: &str = "x-rivet-token";

// Paths
pub const PATH_CONNECT_WEBSOCKET: &str = "/connect";

// WebSocket protocol prefixes
pub const WS_PROTOCOL_STANDARD: &str = "rivet";
pub const WS_PROTOCOL_TARGET: &str = "rivet_target.";
pub const WS_PROTOCOL_ACTOR: &str = "rivet_actor.";
pub const WS_PROTOCOL_ENCODING: &str = "rivet_encoding.";
pub const WS_PROTOCOL_CONN_PARAMS: &str = "rivet_conn_params.";
pub const WS_PROTOCOL_CONN_ID: &str = "rivet_conn.";
pub const WS_PROTOCOL_CONN_TOKEN: &str = "rivet_conn_token.";
pub const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

#[derive(Debug, Clone, Copy)]
pub enum TransportKind {
    WebSocket,
    Sse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingKind {
    Json,
    Cbor,
}

impl EncodingKind {
    pub fn as_str(&self) -> &str {
        match self {
            EncodingKind::Json => "json",
            EncodingKind::Cbor => "cbor",
        }
    }
}

impl ToString for EncodingKind {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}



// Max size of each entry is 128 bytes
pub type ActorKey = Vec<String>;
