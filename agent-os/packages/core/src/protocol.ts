// JSON-RPC 2.0 types and helpers for ACP communication

export interface JsonRpcRequest {
	jsonrpc: "2.0";
	id: number;
	method: string;
	params?: unknown;
}

export interface JsonRpcResponse {
	jsonrpc: "2.0";
	id: number;
	result?: unknown;
	error?: JsonRpcError;
}

export interface JsonRpcError {
	code: number;
	message: string;
	data?: unknown;
}

export interface JsonRpcNotification {
	jsonrpc: "2.0";
	method: string;
	params?: unknown;
}

export function serializeMessage(
	msg: JsonRpcRequest | JsonRpcNotification,
): string {
	return `${JSON.stringify(msg)}\n`;
}

export function deserializeMessage(
	line: string,
): JsonRpcResponse | JsonRpcNotification | null {
	try {
		const parsed = JSON.parse(line);
		if (parsed?.jsonrpc !== "2.0") return null;
		return parsed;
	} catch {
		return null;
	}
}

export function isResponse(
	msg: JsonRpcResponse | JsonRpcNotification,
): msg is JsonRpcResponse {
	return "id" in msg;
}
