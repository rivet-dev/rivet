import { useEffect, useRef, useState } from "react";

export const useWebSocket = (
	url: string,
	protocols?: string | string[],
	handlers?: {
		onMessage?: (event: MessageEvent) => void;
	},
) => {
	const wsRef = useRef<WebSocket | null>(null);
	const [isConnected, setIsConnected] = useState(false);
	const [error, setError] = useState<Error | null>(null);

	const onMessageRef = useRef(handlers?.onMessage);
	useEffect(() => {
		onMessageRef.current = handlers?.onMessage;
	}, [handlers?.onMessage]);

	useEffect(() => {
		const ws = new WebSocket(url, protocols);
		wsRef.current = ws;
		setError(null);

		const onOpen = () => {
			setIsConnected(true);
		};

		const onClose = () => {
			setIsConnected(false);
		};

		const onError = (event: Event) => {
			setIsConnected(false);
			setError(new Error("WebSocket error occurred"));
		};

		const onMessage = (event: MessageEvent) => {
			onMessageRef.current?.(event);
		};

		ws.addEventListener("open", onOpen);
		ws.addEventListener("close", onClose);
		ws.addEventListener("error", onError);
		ws.addEventListener("message", onMessage);

		return () => {
			ws.removeEventListener("open", onOpen);
			ws.removeEventListener("close", onClose);
			ws.removeEventListener("error", onError);
			ws.removeEventListener("message", onMessage);
			ws.close();
		};
	}, [url, protocols]);

	const sendMessage = (...params: Parameters<WebSocket["send"]>) => {
		if (wsRef.current && isConnected) {
			wsRef.current.send(...params);
		} else {
			console.warn("WebSocket is not connected. Unable to send message.");
		}
	};

	return { isConnected, error, sendMessage };
};
