import { createActorInspectorClient } from "@rivetkit/core/inspector";
import {
	type ActorContext,
	createDefaultActorContext,
} from "@/components/actors";
import { ensureTrailingSlash } from "@/lib/utils";

export const createInspectorActorContext = ({
	url,
	token,
	name,
}: {
	url: string;
	token: string;
	name: string;
}) => {
	const def = createDefaultActorContext();
	const newUrl = new URL(url);
	if (!newUrl.pathname.endsWith("registry/inspect")) {
		if (!newUrl.pathname.endsWith("registry")) {
			newUrl.pathname = `${ensureTrailingSlash(newUrl.pathname)}registry`;
		}
		if (!newUrl.pathname.endsWith("inspect")) {
			newUrl.pathname = `${ensureTrailingSlash(newUrl.pathname)}inspect`;
		}
	}
	newUrl.pathname = newUrl.pathname.replace(
		"/registry/inspect",
		"/registry/actors/inspect",
	);
	return {
		...def,
		createActorInspectorFetchConfiguration(actorId) {
			return {
				headers: {
					"X-RivetKit-Query": JSON.stringify({
						getForId: { actorId, name },
					}),
					Authorization: `Bearer ${token}`,
				},
			};
		},
		createActorInspector(actorId) {
			return createActorInspectorClient(
				newUrl.href,
				this.createActorInspectorFetchConfiguration(actorId),
			);
		},
	} satisfies ActorContext;
};
