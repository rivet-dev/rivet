import { queryOptions } from "@tanstack/react-query";
import z from "zod";
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
					if (!opts.url) {
						throw new Error("No inspector URL provided");
					}
					await getInspectorClientEndpoint(opts.url);
					return true;
				},
				enabled: Boolean(opts.url),
			});
		},
	} satisfies DefaultDataProvider;
};

export async function getInspectorClientEndpoint(url: string) {
	const rivetkitEndpoint = url.replace(/\/+$/, "");

	const finalUrl =
		(await tryMetadata(`${rivetkitEndpoint}`)) ||
		(await tryMetadata(`${rivetkitEndpoint}/api/rivet`));

	if (!finalUrl) {
		throw new Error("Failed to reach client endpoint");
	}

	const finalResponse = await fetch(finalUrl, { method: "OPTIONS" });
	if (!finalResponse.ok) {
		throw new Error("Failed to reach client endpoint");
	}

	return finalUrl;
}

const metadataSchema = z.object({
	runtime: z.string().optional(),
	version: z.string().optional(),
	clientEndpoint: z.string().optional(),
	clientNamespace: z.string().optional(),
	clientToken: z.string().optional(),
});

async function tryMetadata(url: string) {
	try {
		const response = await fetch(`${url}/metadata`);
		if (!response.ok) {
			throw new Error("Failed to fetch metadata");
		}

		const data = await response.json();
		const parsed = metadataSchema.parse(data);

		return parsed.clientEndpoint || url;
	} catch {
		return null;
	}
}
