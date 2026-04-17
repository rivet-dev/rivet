import type { Context as HonoContext } from "hono";
import type { RegistryConfig } from "@/registry/config";
import type { EngineControlClient } from "@/engine-client/driver";
import type { AnyClient } from "@/client/client";

function removedRuntimeError(): Error {
	return new Error(
		"The legacy TypeScript actor runtime has been removed. Use Registry.startEnvoy() to run actors through the native rivetkit-core path.",
	);
}

export class EngineActorDriver {
	constructor(
		readonly _config: RegistryConfig,
		readonly _engineClient: EngineControlClient,
		readonly _inlineClient: AnyClient,
	) {}

	async waitForReady(): Promise<void> {}

	async serverlessHandleStart(_c: HonoContext): Promise<Response> {
		throw removedRuntimeError();
	}

	async shutdownRunner(_immediate: boolean): Promise<void> {}
}
