import type { Logger } from "pino";
import type { UnboundedSender } from "antiox/sync/mpsc";
import type { EnvoyConfig } from "./config.js";
import type { EnvoyHandle } from "./handle.js";
import type { ToEnvoyMessage } from "./tasks/envoy/index.js";
import type { WebSocketTxMessage } from "./websocket.js";

export interface SharedContext {
	config: EnvoyConfig;

	/** Unique string identifying this Envoy process. */
	envoyKey: string;

	/** Cached child logger with envoy-specific attributes. */
	logCached?: Logger;

	envoyTx: UnboundedSender<ToEnvoyMessage>;

	/** Handle passed to user callbacks for interacting with actors. */
	handle: EnvoyHandle;

	/** Current websocket sender. Set by connect, undefined between connections. */
	wsTx?: UnboundedSender<WebSocketTxMessage>;
}
