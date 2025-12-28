import {
	type ActorOptions,
	type AnyActorRegistry,
	type CreateRivetKitOptions,
	createRivetKit as createVanillaRivetKit,
} from "@rivetkit/framework-base";
import { useStore } from "@tanstack/react-store";
import { useEffect, useRef } from "react";
import {
	type Client,
	createClient,
	type ExtractActorsFromRegistry,
} from "rivetkit/client";

export { ActorConnDisposed, createClient } from "rivetkit/client";
export { ActorConnStatus } from "@rivetkit/framework-base";

export function createRivetKit<Registry extends AnyActorRegistry>(
	clientInput: Parameters<typeof createClient>[0],
	opts: CreateRivetKitOptions<Registry> = {},
) {
	return createRivetKitWithClient<Registry>(
		createClient<Registry>(clientInput),
		opts,
	);
}

export function createRivetKitWithClient<Registry extends AnyActorRegistry>(
	client: Client<Registry>,
	opts: CreateRivetKitOptions<Registry> = {},
) {
	const { getOrCreateActor } = createVanillaRivetKit<
		Registry,
		ExtractActorsFromRegistry<Registry>
	>(client, opts);

	/**
	 * Hook to connect to a actor and retrieve its state. Using this hook with the same options
	 * will return the same actor instance. This simplifies passing around the actor state in your components.
	 * It also provides a method to listen for events emitted by the actor.
	 * @param opts - Options for the actor, including its name, key, and parameters.
	 * @returns An object containing the actor's state and a method to listen for events.
	 */
	function useActor<
		ActorName extends keyof ExtractActorsFromRegistry<Registry>,
	>(opts: ActorOptions<Registry, ActorName>) {
		// getOrCreateActor syncs opts to store on every call
		const { mount, state } = getOrCreateActor<ActorName>(opts);

		useEffect(() => {
			return mount();
		}, [mount]);

		const actorState = useStore(state) || {};

		/**
		 * Hook to listen for events emitted by the actor.
		 * This hook allows you to subscribe to specific events emitted by the actor and execute a handler function
		 * when the event occurs.
		 * It uses the `useEffect` hook to set up the event listener when the actor connection is established.
		 * It cleans up the listener when the component unmounts or when the actor connection changes.
		 * @param eventName The name of the event to listen for.
		 * @param handler The function to call when the event is emitted.
		 */
		function useEvent(
			eventName: string,
			// biome-ignore lint/suspicious/noExplicitAny: strong typing of handler is not supported yet
			handler: (...args: any[]) => void,
		) {
			const ref = useRef(handler);
			const actorState = useStore(state) || {};

			useEffect(() => {
				ref.current = handler;
			}, [handler]);

			// biome-ignore lint/correctness/useExhaustiveDependencies: it's okay to not include all dependencies here
			useEffect(() => {
				if (!actorState?.connection) return;

				function eventHandler(...args: any[]) {
					ref.current(...args);
				}
				return actorState.connection.on(eventName, eventHandler);
			}, [
				actorState.connection,
				actorState.connStatus,
				actorState.hash,
				eventName,
			]);
		}

		return {
			...actorState,
			useEvent,
		};
	}

	return {
		useActor,
	};
}
