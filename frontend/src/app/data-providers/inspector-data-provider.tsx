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

	const client = createClient(
		opts.url,
		{ token: opts.token || "" },
		{
			// @ts-expect-error
			targetAddressSpace: "loopback",
		},
	);
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

	const nsEngineContext = createNamespaceEngineContext({
		...global,
		namespace: "default",
		engineToken: () => opts.token!,
		client,
	});

	return {
		...global,
		...nsEngineContext,
		endpoint: opts.url || global.endpoint || nsEngineContext.endpoint,
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
