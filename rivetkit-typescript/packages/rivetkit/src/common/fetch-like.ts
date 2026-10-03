type HeadersLike = {
	entries(): IterableIterator<[string, string]>;
};

export type RequestLike = Request & {
	readonly headers: HeadersLike & Request["headers"];
};

export type ResponseLike = Response & {
	readonly headers: HeadersLike & Response["headers"];
};

export function isUrlLike(value: unknown): value is URL {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof (value as URL).href === "string" &&
		typeof (value as URL).pathname === "string" &&
		typeof (value as URL).search === "string"
	);
}

export function isRequestLike(value: unknown): value is RequestLike {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof (value as Request).url === "string" &&
		typeof (value as Request).method === "string" &&
		isHeadersLike((value as Request).headers)
	);
}

export function isResponseLike(value: unknown): value is ResponseLike {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof (value as Response).status === "number" &&
		isHeadersLike((value as Response).headers) &&
		typeof (value as Response).arrayBuffer === "function"
	);
}

function isHeadersLike(value: unknown): value is HeadersLike {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof (value as HeadersLike).entries === "function"
	);
}

/**
 * Reads a request body into bytes. Aborting the request's signal cancels the
 * body and rejects the read.
 */
export function readRequestBody(request: Request): Promise<ArrayBuffer> {
	// Firefox has no Request.body.
	if (!request.body) return request.arrayBuffer();
	return new Response(
		request.body.pipeThrough(new TransformStream(), {
			signal: request.signal,
		}),
	).arrayBuffer();
}
