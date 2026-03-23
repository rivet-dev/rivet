import {
	type ActorOptions,
	type AnyActorRegistry,
	type CreateRivetKitOptions,
	createRivetKit as createVanillaRivetKit,
} from "@rivetkit/framework-base";
import { useStore } from "@tanstack/react-store";
import { use, useEffect, useRef } from "react";
import {
	type ActorConn,
	type Client,
	type ClientConfigInput,
	createClient,
	type ExtractActorsFromRegistry,
} from "rivetkit/client";

export { ActorConnDisposed, createClient } from "rivetkit/client";
export type { ActorConnStatus } from "@rivetkit/framework-base";

export function createRivetKit<Registry extends AnyActorRegistry>(
	clientInput: string | ClientConfigInput | undefined = undefined,
	opts: CreateRivetKitOptions<Registry> = {},
) {
	// @ts-ignore Type instantiation can be excessively deep for complex registries.
	return createRivetKitWithClient<Registry>(
		createClient<Registry>(clientInput),
		opts,
	);
}

export function createRivetKitWithClient<Registry extends AnyActorRegistry>(
	client: Client<Registry>,
	opts: CreateRivetKitOptions<Registry> = {},
) {
	// @ts-ignore Type instantiation can be excessively deep for complex registries.
	const { getOrCreateActor } = createVanillaRivetKit(client, opts);

	function useActor<
		ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
	>(opts: ActorOptions<Registry, ActorName>) {
		const { mount, state, getConnectPromise } = getOrCreateActor<ActorName>(opts);

		useEffect(() => {
			return mount();
		}, [mount]);

		const actorState = useStore(state);

		if (opts.suspense && actorState.connStatus !== "connected") {
			if (actorState.error) {
				throw actorState.error;
			}
			use(getConnectPromise());
		}

		type UseEvent = (typeof actorState)["connection"] extends ActorConn<
			infer AD
		> | null
			? ActorConn<AD>["on"]
			: never;

		const useEvent = ((
			eventName: string,
			handler: (...args: unknown[]) => void,
		) => {
			const ref = useRef(handler);
			const actorState = useStore(state);

			useEffect(() => {
				ref.current = handler;
			}, [handler]);

			// biome-ignore lint/correctness/useExhaustiveDependencies: it's okay to not include all dependencies here
			useEffect(() => {
				const connection = actorState.connection as {
					on: (
						eventName: string,
						callback: (...args: unknown[]) => void,
					) => () => void;
				} | null;
				if (!connection) return;

				function eventHandler(...args: unknown[]) {
					ref.current(...args);
				}
				return connection.on(eventName, eventHandler);
			}, [
				actorState.connection,
				actorState.connStatus,
				actorState.hash,
				eventName,
			]);
		}) as UseEvent;

		return {
			...actorState,
			useEvent,
		};
	}

	return {
		useActor,
	};
}
