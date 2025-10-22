import { check, fail } from "k6";
import http from "k6/http";
import type { Config } from "./types.ts";

type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
type QueryParams = Record<string, string | undefined>;
// Match k6's http.Response type
type ResponseType = {
	status: number;
	body: string | null;
	headers: Record<string, string>;
	[key: string]: unknown;
};
type ExpectedResponse = Record<string, (r: ResponseType) => boolean>;

export function callRivetApi(
	config: Config,
	method: HttpMethod,
	path: string,
	body: unknown = null,
	queryParams: QueryParams = {},
	expectedResponse: ExpectedResponse = {},
) {
	const headers: Record<string, string> = {
		"Content-Type": "application/json",
	};

	// Build URL with query parameters
	let url = config.rivetEndpoint;
	if (!url.endsWith("/") && !path.startsWith("/")) {
		url += "/";
	}
	url += path;

	// Auto-add project and environment if defined
	const defaultParams = {
		namespace: config.rivetNamespace,
		...queryParams,
	};

	// Build query string
	const queryParts: string[] = [];
	for (const [key, value] of Object.entries(defaultParams)) {
		if (value !== undefined) {
			queryParts.push(
				`${encodeURIComponent(key)}=${encodeURIComponent(value)}`,
			);
		}
	}
	if (queryParts.length > 0) {
		url += `?${queryParts.join("&")}`;
	}

	const options = { headers };

	console.debug(`Making ${method} request to ${url}`);
	if (body) {
		console.debug("Request body:", JSON.stringify(body, null, 2));
	}
	console.debug("Request headers:", JSON.stringify(headers, null, 2));

	// biome-ignore lint/suspicious/noExplicitAny: <explanation>
	let response: any;
	try {
		if (
			method === "POST" ||
			method === "PUT" ||
			method === "PATCH" ||
			method === "DELETE"
		) {
			const methodFn = {
				POST: http.post,
				PUT: http.put,
				PATCH: http.patch,
				DELETE: http.del,
			}[method];
			response = methodFn(url, JSON.stringify(body), options);
		} else if (method === "GET") {
			http.get(url, options);
		} else {
			throw new Error(`Unsupported method: ${method}`);
		}

		console.debug(`Response status: ${response.status}`);
		console.debug(
			"Response headers:",
			JSON.stringify(response.headers, null, 2),
		);
		console.debug("Response body:", response.body);

		// Standard checks
		const checks = {
			[`${method} ${path} status is 200`]: (r: ResponseType) =>
				r.status === 200,
			[`${method} ${path} response has valid JSON`]: (
				r: ResponseType,
			) => {
				try {
					if (r.body === null) {
						console.error("Empty response body");
						return false;
					}
					JSON.parse(r.body);
					return true;
				} catch (e) {
					console.error("Invalid JSON response:", r.body);
					return false;
				}
			},
			...expectedResponse,
		};

		const success = check(response as unknown as ResponseType, checks);
		if (!success) {
			console.error(`${path} request failed:`);
			console.error("Status:", response.status);
			console.error("Response body:", response.body);
			fail(`${path} request failed`);
		}

		if (!response.body) {
			throw new Error("Empty response body");
		}
		// Convert bytes to string if needed
		const bodyStr =
			typeof response.body === "string"
				? response.body
				: response.body.toString();
		return JSON.parse(bodyStr);
	} catch (error) {
		console.error("API call failed:");
		if (error instanceof Error) {
			console.error("Error message:", error.message);
			console.error("Error stack:", error.stack);
		} else {
			console.error("Unknown error:", error);
		}
		throw error;
	}
}
