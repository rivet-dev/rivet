import { z } from "zod";
import type { ClientConfig } from "@/client/config";
import { sendHttpRequest } from "@/client/utils";
import { combineUrlPath } from "@/utils";
import { logger } from "./log";
import { RegistryConfig } from "@/registry/config";

// Error class for Engine API errors
export class EngineApiError extends Error {
	constructor(
		public readonly group: string,
		public readonly code: string,
		message?: string,
	) {
		super(message || `Engine API error: ${group}/${code}`);
		this.name = "EngineApiError";
	}
}

// TODO: Remove getEndpoint, but it's used in a lot of places
export function getEndpoint(config: ClientConfig | RegistryConfig) {
	// Endpoint is always defined for ClientConfig (has default in schema).
	// RegistryConfig may not have endpoint if using local manager.
	return config.endpoint ?? "http://127.0.0.1:6420";
}

// Helper function for making API calls
export async function apiCall<TInput = unknown, TOutput = unknown>(
	config: ClientConfig,
	method: "GET" | "POST" | "PUT" | "DELETE",
	path: string,
	body?: TInput,
): Promise<TOutput> {
	const endpoint = getEndpoint(config);
	const url = combineUrlPath(endpoint, path, {
		namespace: config.namespace,
	});

	logger().debug({ msg: "making api call", method, url });

	const headers: Record<string, string> = {
		...config.headers,
	};

	// Add Authorization header if token is provided
	if (config.token) {
		headers.Authorization = `Bearer ${config.token}`;
	}

	return await sendHttpRequest<TInput, TOutput>({
		method,
		url,
		headers,
		body,
		encoding: "json",
		skipParseResponse: false,
		requestVersionedDataHandler: undefined,
		requestVersion: undefined,
		responseVersionedDataHandler: undefined,
		responseVersion: undefined,
		requestZodSchema: z.any() as z.ZodType<TInput>,
		responseZodSchema: z.any() as z.ZodType<TOutput>,
		// Identity conversions (passthrough for generic API calls)
		requestToJson: (value) => value,
		requestToBare: (value) => value,
		responseFromJson: (value) => value,
		responseFromBare: (value) => value,
	});
}
