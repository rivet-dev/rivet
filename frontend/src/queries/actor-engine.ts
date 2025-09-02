import { getConfig } from "@/components";
import {
	type ActorContext,
	createDefaultActorContext,
} from "@/components/actors";

export const createEngineActorContext = ({
	token,
}: {
	token?: string;
} = {}) => {
	const def = createDefaultActorContext();

	return {
		...def,
		createActorInspectorFetchConfiguration(actorId) {
			return {
				headers: {
					"x-rivet-actor": actorId,
					"x-rivet-target": "actor",
					"x-rivet-port": "main",
					...(token ? { authorization: `Bearer ${token}` } : {}),
				},
			};
		},
		createActorInspectorUrl() {
			return new URL(
				`${getConfig().apiUrl}/inspect`,
				window.location.origin,
			).href;
		},
	} satisfies ActorContext;
};
