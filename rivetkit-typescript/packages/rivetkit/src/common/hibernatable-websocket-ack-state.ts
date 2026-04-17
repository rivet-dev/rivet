interface HibernatableWebSocketAckStateEntry {
	serverMessageIndex: number;
	bufferedMessageSize: number;
	pendingAckFromMessageIndex: boolean;
	pendingAckFromBufferSize: boolean;
}

export const HIBERNATABLE_WEBSOCKET_ACK_DEADLINE = 5_000;
export const HIBERNATABLE_WEBSOCKET_BUFFERED_MESSAGE_SIZE_THRESHOLD = 500_000;

export class HibernatableWebSocketAckState {
	#entries = new Map<string, HibernatableWebSocketAckStateEntry>();

	createConnEntry(connId: string, serverMessageIndex: number): void {
		this.#entries.set(connId, {
			serverMessageIndex,
			bufferedMessageSize: 0,
			pendingAckFromMessageIndex: false,
			pendingAckFromBufferSize: false,
		});
	}

	hasConnEntry(connId: string): boolean {
		return this.#entries.has(connId);
	}

	deleteConnEntry(connId: string): void {
		this.#entries.delete(connId);
	}

	recordBufferedMessage(
		connId: string,
		messageLength: number,
		bufferSizeThreshold: number,
	): boolean {
		const entry = this.#entries.get(connId);
		if (!entry) {
			return false;
		}

		entry.bufferedMessageSize += messageLength;
		if (entry.bufferedMessageSize < bufferSizeThreshold) {
			return false;
		}

		entry.bufferedMessageSize = 0;
		entry.pendingAckFromBufferSize = true;
		return true;
	}

	onBeforePersist(connId: string, serverMessageIndex: number): boolean {
		const entry = this.#entries.get(connId);
		if (!entry) {
			return false;
		}

		entry.pendingAckFromMessageIndex =
			serverMessageIndex > entry.serverMessageIndex;
		entry.serverMessageIndex = serverMessageIndex;
		return true;
	}

	consumeAck(connId: string): number | undefined {
		const entry = this.#entries.get(connId);
		if (!entry) {
			return undefined;
		}

		if (
			!entry.pendingAckFromMessageIndex &&
			!entry.pendingAckFromBufferSize
		) {
			return undefined;
		}

		entry.pendingAckFromMessageIndex = false;
		entry.pendingAckFromBufferSize = false;
		entry.bufferedMessageSize = 0;
		return entry.serverMessageIndex;
	}
}

export function handleInboundHibernatableWebSocketMessage(input: {
	connId: string;
	hibernatable: { serverMessageIndex: number };
	messageLength: number;
	rivetMessageIndex: number;
	ackState: HibernatableWebSocketAckState;
	saveState: (opts: { immediate?: boolean; maxWait?: number }) => void;
}): void {
	const {
		connId,
		hibernatable,
		messageLength,
		rivetMessageIndex,
		ackState,
		saveState,
	} = input;

	hibernatable.serverMessageIndex = rivetMessageIndex;

	if (ackState.hasConnEntry(connId)) {
		const shouldPersistImmediately = ackState.recordBufferedMessage(
			connId,
			messageLength,
			HIBERNATABLE_WEBSOCKET_BUFFERED_MESSAGE_SIZE_THRESHOLD,
		);
		if (shouldPersistImmediately) {
			saveState({ immediate: true });
		} else {
			saveState({ maxWait: HIBERNATABLE_WEBSOCKET_ACK_DEADLINE });
		}
		return;
	}

	saveState({ maxWait: HIBERNATABLE_WEBSOCKET_ACK_DEADLINE });
}
