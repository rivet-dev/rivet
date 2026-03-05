import {
	type ActorOptions,
	type AnyActorRegistry,
	type CreateRivetKitOptions,
	createRivetKit as createVanillaRivetKit,
} from "@rivetkit/framework-base";
import { useStore } from "@tanstack/react-store";
import { useEffect, useRef } from "react";
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

	/**
	 * Hook to connect to a actor and retrieve its state. Using this hook with the same options
	 * will return the same actor instance. This simplifies passing around the actor state in your components.
	 * It also provides a method to listen for events emitted by the actor.
	 * @param opts - Options for the actor, including its name, key, and parameters.
	 * @returns An object containing the actor's state and a method to listen for events.
	 */
	function useActor<
		ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
	>(opts: ActorOptions<Registry, ActorName>) {
		// getOrCreateActor syncs opts to store on every call
		const { mount, state } = getOrCreateActor<ActorName>(opts);

		useEffect(() => {
			return mount();
		}, [mount]);

		const actorState = useStore(state);
		type UseEvent = (typeof actorState)["connection"] extends ActorConn<
			infer AD
		> | null
			? ActorConn<AD>["on"]
			: never;

		/**
		 * Hook to listen for events emitted by the actor.
		 * This hook allows you to subscribe to specific events emitted by the actor and execute a handler function
		 * when the event occurs.
		 * It uses the `useEffect` hook to set up the event listener when the actor connection is established.
		 * It cleans up the listener when the component unmounts or when the actor connection changes.
		 * @param eventName The name of the event to listen for.
		 * @param handler The function to call when the event is emitted.
		 */
		const useEvent = ((eventName: string, handler: (...args: unknown[]) => void) => {
			const ref = useRef(handler);
			const actorState = useStore(state);

			useEffect(() => {
				ref.current = handler;
			}, [handler]);

			// biome-ignore lint/correctness/useExhaustiveDependencies: it's okay to not include all dependencies here
			useEffect(() => {
				const connection = actorState.connection as
					| {
							on: (
								eventName: string,
								callback: (...args: unknown[]) => void,
							) => () => void;
					  }
					| null;
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
