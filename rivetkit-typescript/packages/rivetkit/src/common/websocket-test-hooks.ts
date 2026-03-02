import type { UniversalWebSocket } from "./websocket-interface";

export type IndexedWebSocketPayload =
	| string
	| ArrayBufferLike
	| Blob
	| ArrayBufferView;

export type IndexedWebSocketSender = (
	data: IndexedWebSocketPayload,
	rivetMessageIndex?: number,
) => void | Promise<void>;

export interface IndexedWebSocketTestHook {
	__rivetSendWithMessageIndex?: IndexedWebSocketSender;
}

export function setIndexedWebSocketTestSender(
	websocket: UniversalWebSocket,
	sender: IndexedWebSocketSender,
	enabled: boolean,
): void {
	if (!enabled) {
		return;
	}

	(websocket as IndexedWebSocketTestHook).__rivetSendWithMessageIndex = sender;
}

export function getIndexedWebSocketTestSender(
	websocket: UniversalWebSocket,
): IndexedWebSocketSender | undefined {
	return (websocket as IndexedWebSocketTestHook).__rivetSendWithMessageIndex;
}
