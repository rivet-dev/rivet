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
	message: string;
}

export interface ToClientEvent {
	name: string;
	args: unknown[];
}

export interface ToClientError {
	message: string;
}
