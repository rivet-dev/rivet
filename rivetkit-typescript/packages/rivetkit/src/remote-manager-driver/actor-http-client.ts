import type { ClientConfig } from "@/client/config";
import { HEADER_RIVET_TOKEN } from "@/common/actor-router-consts";

export async function sendHttpRequestToGateway(
	runConfig: ClientConfig,
	gatewayUrl: string,
	actorRequest: Request,
): Promise<Response> {
	// Handle body properly based on method and presence
	let bodyToSend: ArrayBuffer | null = null;
	const guardHeaders = buildGuardHeaders(runConfig, actorRequest);

	if (actorRequest.method !== "GET" && actorRequest.method !== "HEAD") {
		if (actorRequest.bodyUsed) {
			throw new Error("Request body has already been consumed");
		}

		// TODO: This buffers the entire request in memory every time. We
		// need to properly implement streaming bodies.
		const reqBody = await actorRequest.arrayBuffer();

		if (reqBody.byteLength !== 0) {
			bodyToSend = reqBody;

			// If this is a streaming request, we need to convert the headers
			// for the basic array buffer
			guardHeaders.delete("transfer-encoding");
			guardHeaders.set("content-length", String(bodyToSend.byteLength));
		}
	}

	const guardRequest = new Request(gatewayUrl, {
		method: actorRequest.method,
		headers: guardHeaders,
		body: bodyToSend,
		signal: actorRequest.signal,
	});

	return mutableResponse(await fetch(guardRequest));
}

function mutableResponse(fetchRes: Response): Response {
	// We cannot return the raw response from `fetch` since the response type is not mutable.
	//
	// In order for middleware to be able to mutate the response, we need to build a new Response object that is mutable.
	return new Response(fetchRes.body, fetchRes);
}

function buildGuardHeaders(
	runConfig: ClientConfig,
	actorRequest: Request,
): Headers {
	const headers = new Headers();
	// Copy all headers from the original request
	actorRequest.headers.forEach((value, key) => {
		headers.set(key, value);
	});
	// Add extra headers from config
	for (const [key, value] of Object.entries(runConfig.headers)) {
		headers.set(key, value as string);
	}
	// Add guard-specific headers
	if (runConfig.token) {
		headers.set(HEADER_RIVET_TOKEN, runConfig.token);
	}
	return headers;
}
