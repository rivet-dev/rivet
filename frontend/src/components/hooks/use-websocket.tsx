import { useCallback, useEffect, useRef, useState } from "react";
import ReconnectingWebSocket from "reconnectingwebsocket";

export type ConnectionStatus =
	| "connecting"
	| "connected"
	| "disconnected"
	| "error";

export const useWebSocket = (
	url: string,
	protocols?: string | string[],
	handlers?: {
		onMessage?: (event: ReconnectingWebSocket.MessageEvent) => void;
	},
) => {
	const wsRef = useRef<ReconnectingWebSocket | null>(null);
	const [status, setStatus] = useState<ConnectionStatus>("disconnected");
	const [error, setError] = useState<Error | null>(null);

	const onMessageRef = useRef(handlers?.onMessage);
	useEffect(() => {
		onMessageRef.current = handlers?.onMessage;
	}, [handlers?.onMessage]);

	// Centralized connection logic so we can manually reconnect.
	const connect = useCallback(() => {
		const { promise, reject, resolve } = Promise.withResolvers<void>();

		// If there's an existing socket, close it first to avoid duplicate listeners.
		if (wsRef.current) {
			try {
				wsRef.current.close(1000, "unmount");
			} catch (e) {
				console.warn(
					"Error closing existing WebSocket before reconnect",
					e,
				);
				reject(e);
			}
		}

		const ws = new ReconnectingWebSocket(url, protocols, {
			binaryType: "arraybuffer",
		});
		wsRef.current = ws;
		setError(null);

		const onOpen = () => {
			setStatus("connected");
			resolve();
		};

		const onClose = () => {
			setStatus("disconnected");
			reject(new Error("WebSocket disconnected"));
		};

		const onError = (event: ReconnectingWebSocket.ErrorEvent) => {
			const error = new Error("WebSocket error occurred", {
				cause: event,
			});
			setStatus("error");
			setError(error);
			reject(error);
		};

		const onMessage = (event: ReconnectingWebSocket.MessageEvent) => {
			onMessageRef.current?.(event);
		};

		const onConnecting = () => {
			setStatus("connecting");
		};

		ws.addEventListener("open", onOpen);
		ws.addEventListener("close", onClose);
		ws.addEventListener("error", onError);
		ws.addEventListener("message", onMessage);
		ws.addEventListener("connecting", onConnecting);

		// Store a cleanup function on the ref for easier manual teardown if needed.
		(ws as any)._cleanup = () => {
			ws.removeEventListener("open", onOpen);
			ws.removeEventListener("close", onClose);
			ws.removeEventListener("error", onError);
			ws.removeEventListener("message", onMessage);
			ws.removeEventListener("connecting", onConnecting);
			try {
				ws.close(1000, "cleanup");
			} catch (e) {
				console.warn("Error closing WebSocket during cleanup", e);
			}
			if (wsRef.current === ws) wsRef.current = null;
		};

		return promise;
	}, [url, protocols]);

	// Initial connect + reconnect when url/protocols change.
	useEffect(() => {
		connect();
		return () => {
			if (wsRef.current && (wsRef.current as any)._cleanup) {
				(wsRef.current as any)._cleanup();
			} else if (wsRef.current) {
				try {
					wsRef.current.close(1000, "unmount");
				} catch (e) {
					console.warn("Error closing WebSocket on unmount", e);
				}
				wsRef.current = null;
			}
		};
	}, [connect]);

	// Manual reconnect function exposed to consumers.
	const reconnect = useCallback(() => {
		connect();
	}, [connect]);

	const sendMessage = useCallback(
		(...params: Parameters<WebSocket["send"]>) => {
			wsRef.current?.send(...params);
		},
		[],
	);

	return { status, error, sendMessage, reconnect };
};
