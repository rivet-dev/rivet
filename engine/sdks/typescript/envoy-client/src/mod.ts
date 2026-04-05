import * as protocol from "@rivetkit/engine-envoy-protocol";
import type { Logger } from "pino";
import type WebSocket from "ws";
import { logger } from "./log.js";
import { importWebSocket } from "./websocket.js";
import {
	v4 as uuidv4,
} from "uuid";
import { inspect } from "util";

export { idToStr, injectLatency } from "./utils";

export interface EnvoyConfig {
	logger?: Logger;
	version: number;
	endpoint: string;
	token?: string;
	namespace: string;
	poolName: string;
	prepopulateActorNames: Record<string, { metadata: Record<string, any> }>;
	metadata?: Record<string, any>;

	/**
	 * Debug option to inject artificial latency (in ms) into WebSocket
	 * communication. Messages are queued and delivered in order after the
	 * configured delay.
	 *
	 * @experimental For testing only.
	 */
	debugLatencyMs?: number;
}

export class Envoy {
	#config: EnvoyConfig;
	#envoyKey: string = uuidv4();
	#ws?: WebSocket;

	#started: boolean = false;

	// Cached child logger with envoy-specific attributes
	#logCached?: Logger;

	constructor(config: EnvoyConfig) {
		this.#config = config;
	}

	#wsUrl() {
		const wsEndpoint = this.#config.endpoint
			.replace("http://", "ws://")
			.replace("https://", "wss://");

		// Ensure the endpoint ends with /runners/connect
		const baseUrl = wsEndpoint.endsWith("/")
			? wsEndpoint.slice(0, -1)
			: wsEndpoint;
		const parameters = [
			['protocol_version', protocol.VERSION],
			['namespace', this.#config.namespace],
			['envoy_key', this.#envoyKey],
			['pool_name', this.#config.poolName],
		];
		return `${baseUrl}/envoys/connect?${parameters.map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join('&')}`;
	}

	async start() {
		if (this.#started) throw new Error("Cannot call envoy.start twice");
		this.#started = true;

		this.log?.info({ msg: "starting envoy" });

		try {
			await this.#connect();
		} catch (error) {
			this.#started = false;
			throw error;
		}
	}

	async #connect() {
		const WS = await importWebSocket();

		// Assertion to clear previous WebSocket
		if (
			this.#ws &&
			(this.#ws.readyState === WS.CONNECTING ||
				this.#ws.readyState === WS.OPEN)
		) {
			this.log?.error(
				"found duplicate ws, closing previous",
			);
			this.#ws.close(1000, "duplicate_websocket");
		}

		const protocols = ["rivet"];
		if (this.#config.token)
			protocols.push(`rivet_token.${this.#config.token}`);


		this.#ws = new WS(this.#wsUrl(), protocols) as any as WebSocket;

		this.log?.info({
			msg: "connecting",
			endpoint: this.#config.endpoint,
			namespace: this.#config.namespace,
			envoyKey: this.#envoyKey,
			hasToken: !!this.#config.token,
		});

		this.#ws.addEventListener("open", () => {
			// Send init message
			const init: protocol.ToRivetInit = {
				envoyKey: this.#envoyKey,
				name: this.#config.poolName,
				version: this.#config.version,
				prepopulateActorNames: new Map(
					Object.entries(this.#config.prepopulateActorNames).map(
						([name, data]) => [
							name,
							{ metadata: JSON.stringify(data.metadata) },
						],
					),
				),
				metadata: JSON.stringify(this.#config.metadata),
			};

			this.#wsSend({
				tag: "ToRivetInit",
				val: init,
			});
		});
	}

	#wsSend(message: protocol.ToRivet) {
		this.log?.debug({
			msg: "sending runner message",
			data: inspect(message),
		});

		const encoded = protocol.encodeToRivet(message);

		// Normally synchronous. When debugLatencyMs is set, the send is
		// deferred but message order is preserved.
		injectLatency(this.#config.debugLatencyMs).then(() => {
			const ws = this.getWsIfReady();
			if (ws) {
				ws.send(encoded);
			} else {
				this.log?.error({
					msg: "WebSocket not available or not open for sending data",
				});
			}
		});
	}

	/** Asserts WebSocket exists and is ready. */
	getWsIfReady(): WebSocket | undefined {
		if (
			!!this.#ws &&
			this.#ws.readyState === 1
		) {
			return this.#ws;
		} else {
			return undefined;
		}
	}

	dispose() {

	}

	get log(): Logger | undefined {
		if (this.#logCached) return this.#logCached;

		const l = logger();
		if (l) {
			// If has connected, create child logger with relevant metadata
			//
			// Otherwise, return default logger
			if (this.#envoyKey) {
				this.#logCached = l.child({
					envoyKey: this.#envoyKey,
				});
				return this.#logCached;
			} else {
				return l;
			}
		}

		return undefined;
	}
}
