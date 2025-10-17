import {
	type ActorContext,
	createDefaultActorContext,
} from "@/components/actors";
import { ensureTrailingSlash } from "@/lib/utils";

export const createInspectorActorContext = ({
	url,
	token: inspectorToken,
	engineToken,
}: {
	url: string;
	token: string | (() => string) | (() => Promise<string>);
	engineToken?: string;
}) => {
	const def = createDefaultActorContext({
		hash: btoa(url + inspectorToken + (engineToken || "")).slice(0, 8),
	});
	const newUrl = new URL(url);
	if (!newUrl.pathname.endsWith("inspect")) {
		newUrl.pathname = `${ensureTrailingSlash(newUrl.pathname)}inspect`;
	}
	return {
		...def,
		async createActorInspectorFetchConfiguration(actorId, opts) {
			return {
				headers: {
					"x-rivet-actor": actorId,
					"x-rivet-target": "actor",
					...(engineToken ? { "x-rivet-token": engineToken } : {}),
					...(opts?.auth
						? {
								...{
									authorization: `Bearer ${
										typeof inspectorToken === "string"
											? inspectorToken
											: await inspectorToken()
									}`,
								},
							}
						: {}),
				},
			};
		},
		createActorInspectorUrl() {
			return new URL(`${url}/inspect`, window.location.origin).href;
		},
	} satisfies ActorContext;
};
