import { assertUnreachable } from "../common/utils.ts";

export type WebSocketMessage = string | Blob | ArrayBuffer | Uint8Array;

export function messageLength(message: WebSocketMessage): number {
	if (message instanceof Blob) {
		return message.size;
	} else if (message instanceof ArrayBuffer) {
		return message.byteLength;
	} else if (message instanceof Uint8Array) {
		return message.byteLength;
	} else if (typeof message === "string") {
		return message.length;
	} else {
		assertUnreachable(message);
	}
}
