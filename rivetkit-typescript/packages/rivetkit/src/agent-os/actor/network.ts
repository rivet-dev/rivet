/**
 * Overview: Network actions for the agent-os HOC — proxies HTTP into the
 * VM via agent-os 0.2.8 `httpRequest` (replaces the removed `fetch(port, Request)`).
 */
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

/** Extract path + query from a URL string for HttpRequest.path. */
function requestPath(url: string): string {
	try {
		const parsed = new URL(url, "http://localhost");
		return `${parsed.pathname}${parsed.search}`;
	} catch {
		return url.startsWith("/") ? url : `/${url}`;
	}
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

			// 0.2.8: fetch(port, Request) → httpRequest({ port, path, ... }).
			const response = await agentOs.httpRequest({
				port,
				path: requestPath(url),
				method: options?.method ?? "GET",
				headers: options?.headers,
				body: options?.body,
			});

			return {
				status: response.status,
				statusText: response.statusText,
				headers: response.headers,
				body: response.body,
			};
		},
	};
}
