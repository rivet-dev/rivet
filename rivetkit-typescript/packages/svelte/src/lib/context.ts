/**
 * Svelte context helpers for sharing a RivetKit instance through the
 * component tree.
 *
 * Follows the type-safe context pattern established by runed and bits-ui.
 *
 * @module
 */

import type {
	AnyActorRegistry,
	CreateRivetKitOptions,
} from "@rivetkit/framework-base";
import type { Client } from "rivetkit/client";
import { createContext, hasContext, setContext } from "svelte";
import {
	type createClient,
	createRivetKit,
	createRivetKitWithClient,
	type RivetKit,
} from "./rivetkit.svelte.js";

export interface RivetContext<Registry extends AnyActorRegistry> {
	set(rivet: RivetKit<Registry>): RivetKit<Registry>;
	get(): RivetKit<Registry>;
	has(): boolean;
	setup(
		clientInput?: Parameters<typeof createClient<Registry>>[0],
		opts?: CreateRivetKitOptions<Registry>,
	): RivetKit<Registry>;
	setupWithClient(
		client: Client<Registry>,
		opts?: CreateRivetKitOptions<Registry>,
	): RivetKit<Registry>;
}

export function createRivetContext<Registry extends AnyActorRegistry>(
	name = "RivetKit",
): RivetContext<Registry> {
	const markerKey = Symbol(name);
	const [unsafeGetContext, unsafeSetContext] =
		createContext<RivetKit<Registry>>();

	function has(): boolean {
		return hasContext(markerKey);
	}

	function get(): RivetKit<Registry> {
		if (!has()) {
			throw new Error(
				`Context "${name}" not found. Create an app-local Rivet context and call ${name}.set(...) or ${name}.setup(...) in a parent layout.`,
			);
		}

		return unsafeGetContext();
	}

	function set(rivet: RivetKit<Registry>): RivetKit<Registry> {
		setContext(markerKey, true);
		return unsafeSetContext(rivet);
	}

	function setup(
		clientInput?: Parameters<typeof createClient<Registry>>[0],
		opts?: CreateRivetKitOptions<Registry>,
	): RivetKit<Registry> {
		return set(createRivetKit<Registry>(clientInput, opts));
	}

	function setupWithClient(
		client: Client<Registry>,
		opts?: CreateRivetKitOptions<Registry>,
	): RivetKit<Registry> {
		return set(createRivetKitWithClient<Registry>(client, opts));
	}

	return {
		set,
		get,
		has,
		setup,
		setupWithClient,
	};
}
