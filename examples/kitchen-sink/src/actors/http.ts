import type { ActorContext } from "rivetkit";

export function handleHttpRequest(
	c: ActorContext<any, any, any, any, any, any>,
	request: Request,
) {
	const url = new URL(request.url);
	const method = request.method;
	const path = url.pathname;

	// Track request
	if (!c.state.requestCount) c.state.requestCount = 0;
	if (!c.state.requestHistory) c.state.requestHistory = [];

	c.state.requestCount++;
	c.state.requestHistory.push({
		method,
		path,
		timestamp: Date.now(),
		headers: Object.fromEntries(request.headers.entries()),
	});

	c.log.info({
		msg: "http request received",
		method,
		path,
		fullUrl: request.url,
		requestCount: c.state.requestCount,
	});

	// Handle different endpoints
	if (path === "/api/hello") {
		return new Response(
			JSON.stringify({
				message: "Hello from HTTP actor!",
				timestamp: Date.now(),
				requestCount: c.state.requestCount,
			}),
			{
				headers: { "Content-Type": "application/json" },
			},
		);
	}

	if (path === "/api/echo" && method === "POST") {
		return new Response(request.body, {
			headers: {
				"Content-Type":
					request.headers.get("Content-Type") || "text/plain",
				"X-Echo-Timestamp": Date.now().toString(),
			},
		});
	}

	if (path === "/api/stats") {
		return new Response(
			JSON.stringify({
				requestCount: c.state.requestCount,
				requestHistory: c.state.requestHistory.slice(-10), // Last 10 requests
			}),
			{
				headers: { "Content-Type": "application/json" },
			},
		);
	}

	if (path === "/api/headers") {
		const headers = Object.fromEntries(request.headers.entries());
		return new Response(
			JSON.stringify({
				headers,
				method,
				path,
				timestamp: Date.now(),
			}),
			{
				headers: { "Content-Type": "application/json" },
			},
		);
	}

	if (path === "/api/json" && method === "POST") {
		return request.json().then((body) => {
			return new Response(
				JSON.stringify({
					received: body,
					method,
					timestamp: Date.now(),
					processed: true,
				}),
				{
					headers: { "Content-Type": "application/json" },
				},
			);
		});
	}

	// Handle custom paths with query parameters
	if (path.startsWith("/api/custom")) {
		const searchParams = Object.fromEntries(url.searchParams.entries());
		return new Response(
			JSON.stringify({
				path,
				method,
				queryParams: searchParams,
				timestamp: Date.now(),
				message: "Custom endpoint response",
			}),
			{
				headers: { "Content-Type": "application/json" },
			},
		);
	}

	// Return 404 for unhandled paths
	return new Response(
		JSON.stringify({
			error: "Not Found",
			path,
			method,
			availableEndpoints: [
				"/api/hello",
				"/api/echo",
				"/api/stats",
				"/api/headers",
				"/api/json",
				"/api/custom/*",
			],
		}),
		{
			status: 404,
			headers: { "Content-Type": "application/json" },
		},
	);
}

export const httpActions = {
	getHttpStats: (c: any) => {
		return {
			requestCount: c.state.requestCount || 0,
			requestHistory: c.state.requestHistory || [],
		};
	},
	clearHttpHistory: (c: any) => {
		c.state.requestHistory = [];
		c.state.requestCount = 0;
		return true;
	},
};
