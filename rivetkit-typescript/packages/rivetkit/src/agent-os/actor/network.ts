import type { AgentOsActorConfig } from "../config";
import type { AgentOsActionContext } from "../types";
import { ensureVm } from "./index";

// Serializable fetch options for the actor action boundary.
export interface VmFetchOptions {
	method?: string;
	headers?: Record<string, string>;
	body?: string | Uint8Array;
}

// Serializable fetch result returned by the actor action.
export interface VmFetchResult {
	status: number;
	statusText: string;
	headers: Record<string, string>;
	body: Uint8Array;
}

// Build network actions for the actor factory.
export function buildNetworkActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	return {
		vmFetch: async (
			c: AgentOsActionContext<TConnParams>,
			port: number,
			url: string,
			options?: VmFetchOptions,
		): Promise<VmFetchResult> => {
			const agentOs = await ensureVm(c, config);

			const headers = new Headers(options?.headers);
			const request = new Request(url, {
				method: options?.method ?? "GET",
				headers,
				body: options?.body ?? null,
			});

			const response = await agentOs.fetch(port, request);

			// Serialize response headers to a plain object.
			const responseHeaders: Record<string, string> = {};
			response.headers.forEach((value, key) => {
				responseHeaders[key] = value;
			});

			const body = new Uint8Array(await response.arrayBuffer());

			return {
				status: response.status,
				statusText: response.statusText,
				headers: responseHeaders,
				body,
			};
		},
	};
}
