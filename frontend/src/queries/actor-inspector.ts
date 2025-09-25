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
	token: string;
	engineToken?: string;
}) => {
	const def = createDefaultActorContext();
	const newUrl = new URL(url);
	if (!newUrl.pathname.endsWith("inspect")) {
		newUrl.pathname = `${ensureTrailingSlash(newUrl.pathname)}inspect`;
	}
	return {
		...def,
		createActorInspectorFetchConfiguration(actorId) {
			return {
				headers: {
					"x-rivet-actor": actorId,
					"x-rivet-target": "actor",
					...(engineToken ? { "x-rivet-token": engineToken } : {}),
					...(inspectorToken
						? { authorization: `Bearer ${inspectorToken}` }
						: {}),
				},
			};
		},
		createActorInspectorUrl() {
			return new URL(`${url}/inspect`, window.location.origin).href;
		},
	} satisfies ActorContext;
};
