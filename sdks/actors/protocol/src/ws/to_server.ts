export interface ToServer {
	body: { rpcRequest: RpcRequest } | { subscriptionRequest: SubscriptionRequest },
}

export interface RpcRequest {
	id: string;
	name: string;
	args: unknown[];
}

export interface SubscriptionRequest {
	eventName: string;
	subscribe: boolean;
}

