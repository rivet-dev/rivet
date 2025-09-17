import {
	type ActorContext,
	createDefaultActorContext,
} from "@/components/actors";
import { ensureTrailingSlash } from "@/lib/utils";

export const createInspectorActorContext = ({
	url,
	token,
}: {
	url: string;
	token: string;
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
					...(token ? { authorization: `Bearer ${token}` } : {}),
				},
			};
		},
		createActorInspectorUrl() {
			return new URL(`${url}/inspect`, window.location.origin).href;
		},
	} satisfies ActorContext;
};
