import { actor } from "rivetkit";

/**
 * Test fixture to verify request object access in all lifecycle hooks
 */
export const requestAccessActor = actor({
	state: {
		// Track request info from different hooks
		onBeforeConnectRequest: {
			hasRequest: false,
			requestUrl: null as string | null,
			requestMethod: null as string | null,
			requestHeaders: {} as Record<string, string>,
		},
		createConnStateRequest: {
			hasRequest: false,
			requestUrl: null as string | null,
			requestMethod: null as string | null,
			requestHeaders: {} as Record<string, string>,
		},
		onRequestRequest: {
			hasRequest: false,
			requestUrl: null as string | null,
			requestMethod: null as string | null,
			requestHeaders: {} as Record<string, string>,
		},
		onWebSocketRequest: {
			hasRequest: false,
			requestUrl: null as string | null,
			requestMethod: null as string | null,
			requestHeaders: {} as Record<string, string>,
		},
	},
	createConnState: (c, { request }, params: { trackRequest?: boolean }) => {
		// In createConnState, the state isn't available yet.

		return {
			trackRequest: params?.trackRequest || false,
			requestInfo:
				params?.trackRequest && request
					? {
							hasRequest: true,
							requestUrl: request.url,
							requestMethod: request.method,
							requestHeaders: Object.fromEntries(
								request.headers.entries(),
							),
						}
					: null,
		};
	},
	onConnect: (c, conn) => {
		// Copy request info from connection state if it was tracked
		if (conn.state.requestInfo) {
			c.state.createConnStateRequest = conn.state.requestInfo;
		}
	},
	onBeforeConnect: (c, { request }, params) => {
		if (params?.trackRequest) {
			if (request) {
				c.state.onBeforeConnectRequest.hasRequest = true;
				c.state.onBeforeConnectRequest.requestUrl = request.url;
				c.state.onBeforeConnectRequest.requestMethod = request.method;

				// Store select headers
				const headers: Record<string, string> = {};
				request.headers.forEach((value, key) => {
					headers[key] = value;
				});
				c.state.onBeforeConnectRequest.requestHeaders = headers;
			} else {
				// Track that we tried but request was not available
				c.state.onBeforeConnectRequest.hasRequest = false;
			}
		}
	},
	onRequest: (c, request) => {
		// Store request info
		c.state.onRequestRequest.hasRequest = true;
		c.state.onRequestRequest.requestUrl = request.url;
		c.state.onRequestRequest.requestMethod = request.method;

		// Store select headers
		const headers: Record<string, string> = {};
		request.headers.forEach((value, key) => {
			headers[key] = value;
		});
		c.state.onRequestRequest.requestHeaders = headers;

		// Return response with request info
		return new Response(
			JSON.stringify({
				hasRequest: true,
				requestUrl: request.url,
				requestMethod: request.method,
				requestHeaders: headers,
			}),
			{
				status: 200,
				headers: { "Content-Type": "application/json" },
			},
		);
	},
	onWebSocket: (c, websocket, { request }) => {
		// Store request info
		c.state.onWebSocketRequest.hasRequest = true;
		c.state.onWebSocketRequest.requestUrl = request.url;
		c.state.onWebSocketRequest.requestMethod = request.method;

		// Store select headers
		const headers: Record<string, string> = {};
		request.headers.forEach((value, key) => {
			headers[key] = value;
		});
		c.state.onWebSocketRequest.requestHeaders = headers;

		// Send request info on connection
		websocket.send(
			JSON.stringify({
				hasRequest: true,
				requestUrl: request.url,
				requestMethod: request.method,
				requestHeaders: headers,
			}),
		);

		// Echo messages back
		websocket.addEventListener("message", (event) => {
			websocket.send(event.data);
		});
	},
	actions: {
		getRequestInfo: (c) => {
			return {
				onBeforeConnect: c.state.onBeforeConnectRequest,
				createConnState: c.state.createConnStateRequest,
				onRequest: c.state.onRequestRequest,
				onWebSocket: c.state.onWebSocketRequest,
			};
		},
	},
});
