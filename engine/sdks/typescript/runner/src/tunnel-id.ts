import * as protocol from "@rivetkit/engine-runner-protocol";

// Type aliases for the message ID components
export type GatewayId = ArrayBuffer;
export type RequestId = ArrayBuffer;
export type MessageIndex = number;
export type MessageId = ArrayBuffer;

/**
 * Build a MessageId from its components
 */
export function buildMessageId(
	gatewayId: GatewayId,
	requestId: RequestId,
	messageIndex: MessageIndex,
): MessageId {
	if (gatewayId.byteLength !== 4) {
		throw new Error(
			`invalid gateway id length: expected 4 bytes, got ${gatewayId.byteLength}`,
		);
	}
	if (requestId.byteLength !== 4) {
		throw new Error(
			`invalid request id length: expected 4 bytes, got ${requestId.byteLength}`,
		);
	}
	if (messageIndex < 0 || messageIndex > 0xffff) {
		throw new Error(
			`invalid message index: must be u16, got ${messageIndex}`,
		);
	}

	const parts: protocol.MessageIdParts = {
		gatewayId,
		requestId,
		messageIndex,
	};

	const encoded = protocol.encodeMessageIdParts(parts);

	if (encoded.byteLength !== 10) {
		throw new Error(
			`message id serialization produced wrong size: expected 10 bytes, got ${encoded.byteLength}`,
		);
	}

	// Create a new ArrayBuffer from the Uint8Array
	const messageId = new ArrayBuffer(10);
	new Uint8Array(messageId).set(encoded);
	return messageId;
}

/**
 * Parse a MessageId into its components
 */
export function parseMessageId(messageId: MessageId): protocol.MessageIdParts {
	if (messageId.byteLength !== 10) {
		throw new Error(
			`invalid message id length: expected 10 bytes, got ${messageId.byteLength}`,
		);
	}
	const uint8Array = new Uint8Array(messageId);
	return protocol.decodeMessageIdParts(uint8Array);
}

/**
 * Convert a GatewayId to a base64 string
 */
export function gatewayIdToString(gatewayId: GatewayId): string {
	const uint8Array = new Uint8Array(gatewayId);
	return bufferToBase64(uint8Array);
}

/**
 * Convert a RequestId to a base64 string
 */
export function requestIdToString(requestId: RequestId): string {
	const uint8Array = new Uint8Array(requestId);
	return bufferToBase64(uint8Array);
}

/**
 * Convert a MessageId to a base64 string
 */
export function messageIdToString(messageId: MessageId): string {
	const uint8Array = new Uint8Array(messageId);
	return bufferToBase64(uint8Array);
}

// Helper functions for base64 encoding/decoding

function bufferToBase64(buffer: Uint8Array): string {
	// Use Node.js Buffer if available, otherwise use browser btoa
	if (typeof Buffer !== "undefined") {
		return Buffer.from(buffer).toString("base64");
	} else {
		// Browser environment
		let binary = "";
		for (let i = 0; i < buffer.byteLength; i++) {
			binary += String.fromCharCode(buffer[i]);
		}
		return btoa(binary);
	}
}
