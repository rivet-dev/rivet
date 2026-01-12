import { actor, type RivetMessageEvent } from "rivetkit";

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
	createConnState: (c, params: { trackRequest?: boolean }) => {
		// In createConnState, the state isn't available yet.

		let requestInfo: {
			hasRequest: boolean;
			requestUrl: string;
			requestMethod: string;
			requestHeaders: Record<string, string>;
		} | null = null;

		if (params?.trackRequest && c.request) {
			const headers: Record<string, string> = {};
			c.request.headers.forEach((value, key) => {
				headers[key] = value;
			});
			requestInfo = {
				hasRequest: true,
				requestUrl: c.request.url,
				requestMethod: c.request.method,
				requestHeaders: headers,
			};
		}

		return {
			trackRequest: params?.trackRequest || false,
			requestInfo,
		};
	},
	onConnect: (c, conn) => {
		// Copy request info from connection state if it was tracked
		if (conn.state.requestInfo) {
			c.state.createConnStateRequest = conn.state.requestInfo;
		}
	},
	onBeforeConnect: (c, params) => {
		if (params?.trackRequest) {
			if (c.request) {
				c.state.onBeforeConnectRequest.hasRequest = true;
				c.state.onBeforeConnectRequest.requestUrl = c.request.url;
				c.state.onBeforeConnectRequest.requestMethod = c.request.method;

				// Store select headers
				const headers: Record<string, string> = {};
				c.request.headers.forEach((value, key) => {
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
	onWebSocket: (c, websocket) => {
		if (!c.request) throw "Missing request";
		// Store request info
		c.state.onWebSocketRequest.hasRequest = true;
		c.state.onWebSocketRequest.requestUrl = c.request.url;
		c.state.onWebSocketRequest.requestMethod = c.request.method;

		// Store select headers
		const headers: Record<string, string> = {};
		c.request.headers.forEach((value, key) => {
			headers[key] = value;
		});
		c.state.onWebSocketRequest.requestHeaders = headers;

		// Send request info on connection
		websocket.send(
			JSON.stringify({
				hasRequest: true,
				requestUrl: c.request.url,
				requestMethod: c.request.method,
				requestHeaders: headers,
			}),
		);

		// Echo messages back
		websocket.addEventListener("message", (event: RivetMessageEvent) => {
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
