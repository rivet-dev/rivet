import { actor, type RivetMessageEvent } from "rivetkit";

type RequestInfo = {
	hasRequest: boolean;
	requestUrl: string | null;
	requestMethod: string | null;
	requestHeaders: Record<string, string>;
};

function emptyRequestInfo(): RequestInfo {
	return {
		hasRequest: false,
		requestUrl: null,
		requestMethod: null,
		requestHeaders: {},
	};
}

function describeRequest(request: Request): RequestInfo {
	const headers: Record<string, string> = {};
	request.headers.forEach((value, key) => {
		headers[key] = value;
	});
	return {
		hasRequest: true,
		requestUrl: request.url,
		requestMethod: request.method,
		requestHeaders: headers,
	};
}

/**
 * Test fixture to verify request object access in all lifecycle hooks
 */
export const requestAccessActor = actor({
	// Track request info from different hooks
	createVars: () => ({
		onBeforeConnectRequest: emptyRequestInfo(),
		onConnectRequest: emptyRequestInfo(),
		onRequestRequest: emptyRequestInfo(),
		onWebSocketRequest: emptyRequestInfo(),
	}),
	onBeforeConnect: (c, params: { trackRequest?: boolean }) => {
		if (params?.trackRequest) {
			if (c.request) {
				c.vars.onBeforeConnectRequest = describeRequest(c.request);
			} else {
				// Track that we tried but request was not available
				c.vars.onBeforeConnectRequest.hasRequest = false;
			}
		}
	},
	onConnect: (c, conn) => {
		if (conn.params?.trackRequest && c.request) {
			c.vars.onConnectRequest = describeRequest(c.request);
		}
	},
	onRequest: (c, request) => {
		// Store request info
		const info = describeRequest(request);
		c.vars.onRequestRequest = info;

		// Return response with request info
		return new Response(JSON.stringify(info), {
			status: 200,
			headers: { "Content-Type": "application/json" },
		});
	},
	onWebSocket: (c, websocket) => {
		if (!c.request) throw "Missing request";
		// Store request info
		const info = describeRequest(c.request);
		c.vars.onWebSocketRequest = info;

		// Send request info on connection
		websocket.send(JSON.stringify(info));

		// Echo messages back
		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			websocket.send(event.data);
		});
	},
	actions: {
		getRequestInfo: (c) => {
			return {
				onBeforeConnect: c.vars.onBeforeConnectRequest,
				onConnect: c.vars.onConnectRequest,
				onRequest: c.vars.onRequestRequest,
				onWebSocket: c.vars.onWebSocketRequest,
			};
		},
	},
});
