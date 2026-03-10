import type { UniversalWebSocket } from "./websocket-interface";

const UTF8_DECODER = new TextDecoder();

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
	__rivetGetHibernatableAckState?: () => HibernatableWebSocketAckStateSnapshot;
	__rivetWaitForHibernatableAck?: (
		serverMessageIndex: number,
	) => Promise<void>;
}

export interface HibernatableWebSocketAckStateSnapshot {
	lastSentIndex: number;
	lastAckedIndex: number;
	pendingIndexes: number[];
}

interface RemoteHibernatableWebSocketAckHooks {
	getState: () => HibernatableWebSocketAckStateSnapshot;
	waitForAck: (serverMessageIndex: number) => Promise<void>;
}

const REMOTE_HIBERNATABLE_WEBSOCKET_ACK_HOOKS = new Map<
	string,
	RemoteHibernatableWebSocketAckHooks
>();

export interface HibernatableWebSocketAckStateTestRequest {
	__rivetkitTestHibernatableAckStateV1: true;
}

export function setIndexedWebSocketTestSender(
	websocket: UniversalWebSocket,
	sender: IndexedWebSocketSender,
	enabled: boolean,
): void {
	if (!enabled) {
		return;
	}

	(websocket as IndexedWebSocketTestHook).__rivetSendWithMessageIndex =
		sender;
}

export function getIndexedWebSocketTestSender(
	websocket: UniversalWebSocket,
): IndexedWebSocketSender | undefined {
	return (websocket as IndexedWebSocketTestHook).__rivetSendWithMessageIndex;
}

export function setHibernatableWebSocketAckTestHooks(
	websocket: UniversalWebSocket,
	hooks: {
		getState: () => HibernatableWebSocketAckStateSnapshot;
		waitForAck: (serverMessageIndex: number) => Promise<void>;
	},
	enabled: boolean,
): void {
	if (!enabled) {
		return;
	}

	const testWebSocket = websocket as IndexedWebSocketTestHook;
	testWebSocket.__rivetGetHibernatableAckState = hooks.getState;
	testWebSocket.__rivetWaitForHibernatableAck = hooks.waitForAck;
}

export function getHibernatableWebSocketAckState(
	websocket: UniversalWebSocket,
): HibernatableWebSocketAckStateSnapshot | undefined {
	return (
		websocket as IndexedWebSocketTestHook
	).__rivetGetHibernatableAckState?.();
}

export function registerRemoteHibernatableWebSocketAckHooks(
	token: string,
	hooks: RemoteHibernatableWebSocketAckHooks,
	enabled: boolean,
): void {
	if (!enabled) {
		return;
	}

	REMOTE_HIBERNATABLE_WEBSOCKET_ACK_HOOKS.set(token, hooks);
}

export function unregisterRemoteHibernatableWebSocketAckHooks(
	token: string | undefined,
	enabled: boolean,
): void {
	if (!enabled || !token) {
		return;
	}

	REMOTE_HIBERNATABLE_WEBSOCKET_ACK_HOOKS.delete(token);
}

export function setRemoteHibernatableWebSocketAckTestHooks(
	websocket: UniversalWebSocket,
	token: string,
	enabled: boolean,
): void {
	if (!enabled) {
		return;
	}

	setHibernatableWebSocketAckTestHooks(
		websocket,
		{
			getState: () => {
				const hooks =
					REMOTE_HIBERNATABLE_WEBSOCKET_ACK_HOOKS.get(token);
				if (!hooks) {
					throw new Error(
						`remote hibernatable websocket ack hooks are unavailable for token ${token}`,
					);
				}
				return hooks.getState();
			},
			waitForAck: async (serverMessageIndex) => {
				const hooks =
					REMOTE_HIBERNATABLE_WEBSOCKET_ACK_HOOKS.get(token);
				if (!hooks) {
					throw new Error(
						`remote hibernatable websocket ack hooks are unavailable for token ${token}`,
					);
				}
				await hooks.waitForAck(serverMessageIndex);
			},
		},
		enabled,
	);
}

export async function waitForHibernatableWebSocketAck(
	websocket: UniversalWebSocket,
	serverMessageIndex: number,
): Promise<void> {
	const waitForAck = (websocket as IndexedWebSocketTestHook)
		.__rivetWaitForHibernatableAck;
	if (!waitForAck) {
		throw new Error(
			"hibernatable websocket ack test hook is unavailable on this transport",
		);
	}

	await waitForAck(serverMessageIndex);
}

export function parseHibernatableWebSocketAckStateTestRequest(
	data: IndexedWebSocketPayload,
	enabled: boolean,
): HibernatableWebSocketAckStateTestRequest | undefined {
	if (!enabled) {
		return undefined;
	}

	let rawText: string | undefined;
	if (typeof data === "string") {
		rawText = data;
	} else if (ArrayBuffer.isView(data)) {
		rawText = UTF8_DECODER.decode(
			new Uint8Array(data.buffer, data.byteOffset, data.byteLength),
		);
	} else if (data instanceof ArrayBuffer) {
		rawText = UTF8_DECODER.decode(data);
	} else if (
		data &&
		typeof data === "object" &&
		"byteLength" in data &&
		typeof data.byteLength === "number"
	) {
		rawText = UTF8_DECODER.decode(new Uint8Array(data as ArrayBufferLike));
	}
	if (rawText === undefined) {
		return undefined;
	}

	let parsed: unknown;
	try {
		parsed = JSON.parse(rawText);
	} catch {
		return undefined;
	}

	if (
		!parsed ||
		typeof parsed !== "object" ||
		(parsed as Partial<HibernatableWebSocketAckStateTestRequest>)
			.__rivetkitTestHibernatableAckStateV1 !== true
	) {
		return undefined;
	}

	return {
		__rivetkitTestHibernatableAckStateV1: true,
	};
}

export function buildHibernatableWebSocketAckStateTestResponse(
	state: HibernatableWebSocketAckStateSnapshot,
	enabled: boolean,
): string | undefined {
	if (!enabled) {
		return undefined;
	}

	return JSON.stringify({
		__rivetkitTestHibernatableAckStateV1: true,
		lastSentIndex: state.lastSentIndex,
		lastAckedIndex: state.lastAckedIndex,
		pendingIndexes: state.pendingIndexes,
	});
}
