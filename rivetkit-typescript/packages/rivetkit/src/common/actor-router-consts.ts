// NOTE: This is in a separate file from the router since it needs to be shared between the client & the server. If this was in the router file, the client would end up importing the *entire* actor router and tree shaking would not work.

// MARK: Paths
export const PATH_CONNECT = "/connect";
export const PATH_WEBSOCKET_PREFIX = "/websocket/";

// MARK: Headers
export const HEADER_ACTOR_QUERY = "x-rivet-query";

export const HEADER_ENCODING = "x-rivet-encoding";

// IMPORTANT: Params must be in headers or in an E2EE part of the request (i.e. NOT the URL or query string) in order to ensure that tokens can be securely passed in params.
export const HEADER_CONN_PARAMS = "x-rivet-conn-params";

export const HEADER_ACTOR_ID = "x-rivet-actor";

export const HEADER_RIVET_TOKEN = "x-rivet-token";

// MARK: Manager Gateway Headers
export const HEADER_RIVET_TARGET = "x-rivet-target";
export const HEADER_RIVET_ACTOR = "x-rivet-actor";
export const HEADER_RIVET_NAMESPACE = "x-rivet-namespace";

// MARK: WebSocket Protocol Prefixes
/** Some servers (such as node-ws & Cloudflare) require explicitly match a certain WebSocket protocol. This gives us a static protocol to match against. */
export const WS_PROTOCOL_STANDARD = "rivet";
export const WS_PROTOCOL_TARGET = "rivet_target.";
export const WS_PROTOCOL_ACTOR = "rivet_actor.";
export const WS_PROTOCOL_ENCODING = "rivet_encoding.";
export const WS_PROTOCOL_CONN_PARAMS = "rivet_conn_params.";
export const WS_PROTOCOL_TOKEN = "rivet_token.";

// MARK: WebSocket Inline Test Protocol Prefixes
export const WS_TEST_PROTOCOL_PATH = "test_path.";

/**
 * Headers that publics can send from public clients.
 *
 * Used for CORS.
 **/
export const ALLOWED_PUBLIC_HEADERS = [
	"Content-Type",
	"User-Agent",
	HEADER_ACTOR_QUERY,
	HEADER_ENCODING,
	HEADER_CONN_PARAMS,
	HEADER_ACTOR_ID,
	HEADER_RIVET_TARGET,
	HEADER_RIVET_ACTOR,
	HEADER_RIVET_NAMESPACE,
	HEADER_RIVET_TOKEN,
];
