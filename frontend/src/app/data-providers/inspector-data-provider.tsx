import { queryOptions } from "@tanstack/react-query";
import {
	createDefaultGlobalContext,
	type DefaultDataProvider,
} from "./default-data-provider";
import {
	createClient,
	createGlobalContext as createGlobalEngineContext,
	createNamespaceContext as createNamespaceEngineContext,
} from "./engine-data-provider";

export const createGlobalContext = (opts: { url?: string; token?: string }) => {
	const def = createDefaultGlobalContext();

	if (!opts.url) {
		return {
			...def,
			endpoint: opts.url,
			features: {
				canCreateActors: false,
				canDeleteActors: false,
			},
		};
	}
	const client = createClient(opts.url, { token: opts.token || "" });
	const global = {
		...def,
		endpoint: opts.url,
		features: {
			canCreateActors: true,
			canDeleteActors: false,
		},
		...createGlobalEngineContext({
			engineToken: () => opts.token!,
		}),
	};

	return {
		...global,
		...createNamespaceEngineContext({
			...global,
			namespace: "default",
			engineToken: () => opts.token!,
			client,
		}),
		statusQueryOptions() {
			return queryOptions({
				...global.statusQueryOptions(),
				queryFn: async () => {
					const response = await fetch(opts.url || "");
					if (!response.ok) {
						throw new Error("Failed to fetch status");
					}
					return true;
				},
				enabled: Boolean(opts.url),
			});
		},
	} satisfies DefaultDataProvider;
};
