import { type ActorContext, actor } from "rivetkit";

export const rawHttpRequestPropertiesActor = actor({
	actions: {},
	onRequest(
		ctx: ActorContext<any, any, any, any, any, any>,
		request: Request,
	) {
		// Extract all relevant Request properties
		const url = new URL(request.url);
		const method = request.method;

		// Get all headers
		const headers = Object.fromEntries(request.headers.entries());

		// Handle body based on content type
		const handleBody = async () => {
			if (!request.body) {
				return null;
			}

			const contentType = request.headers.get("content-type") || "";

			try {
				if (contentType.includes("application/json")) {
					const text = await request.text();
					return text ? JSON.parse(text) : null;
				} else {
					// For non-JSON, return as text
					const text = await request.text();
					return text || null; // Return null for empty bodies
				}
			} catch (error) {
				// If body parsing fails, return null
				return null;
			}
		};

		// Special handling for HEAD and OPTIONS requests
		if (method === "HEAD") {
			return new Response(null, {
				status: 200,
			});
		}

		if (method === "OPTIONS") {
			return new Response(null, {
				status: 204,
			});
		}

		// Return all request properties as JSON
		return handleBody().then((body) => {
			const responseData = {
				// URL properties
				url: request.url,
				pathname: url.pathname,
				search: url.search,
				searchParams: Object.fromEntries(url.searchParams.entries()),
				hash: url.hash,

				// Method
				method: request.method,

				// Headers
				headers: headers,

				// Body
				body,
				bodyText:
					typeof body === "string"
						? body
						: body === null && request.body !== null
							? ""
							: null,

				// Additional properties that might be available
				// Note: Some properties like cache, credentials, mode, etc.
				// might not be available in all environments
				cache: request.cache || null,
				credentials: request.credentials || null,
				mode: request.mode || null,
				redirect: request.redirect || null,
				referrer: request.referrer || null,
			};

			return new Response(JSON.stringify(responseData), {
				headers: { "Content-Type": "application/json" },
			});
		});
	},
});
