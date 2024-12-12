export interface ToClient {
	body:
		| { rpcResponseOk: RpcResponseOk }
		| { rpcResponseError: RpcResponseError }
		| { event: ToClientEvent }
		| { error: ToClientError };
}

export interface RpcResponseOk {
	id: string;
	output: unknown;
}

export interface RpcResponseError {
	id: string;
	code: string;
	message: string;
	metadata?: unknown;
}

export interface ToClientEvent {
	name: string;
	args: unknown[];
}

export interface ToClientError {
	code: string;
	message: string;
	metadata?: unknown;
}
