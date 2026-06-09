import { fileURLToPath } from "node:url";
import { setup } from "rivetkit";
import { dynamicActor, type DynamicActorOptions } from "rivetkit/dynamic";
import { registry as staticRegistry } from "./registry-static";

// Dynamic mirror of the static driver fixture registry: every actor becomes a
// dynamicActor whose loader returns source that re-exports the matching
// static definition. This verifies that loader-resolved actors behave like
// statically registered ones. The source imports the static registry by
// absolute path so the dev child bundler includes it in the bundle.

const staticModulePath = fileURLToPath(
	new URL("./registry-static.ts", import.meta.url),
);

function dynamicOptionsFor(definition: {
	config: Record<string, any>;
}): DynamicActorOptions {
	const config = definition.config;
	const options = (config.options ?? {}) as Record<string, unknown>;
	const canHibernate = options.canHibernateWebSocket;
	return {
		database: config.db !== undefined,
		// Function-valued hibernation predicates evaluate per request in the
		// child; statically enable hibernation support for those actors.
		canHibernateWebSocket:
			typeof canHibernate === "boolean"
				? canHibernate
				: canHibernate !== undefined
					? true
					: undefined,
		actionTimeout: options.actionTimeout as number | undefined,
		sleepTimeout: options.sleepTimeout as number | undefined,
		sleepGracePeriod: options.sleepGracePeriod as number | undefined,
		noSleep: options.noSleep as boolean | undefined,
		maxQueueSize: options.maxQueueSize as number | undefined,
		maxQueueMessageSize: options.maxQueueMessageSize as number | undefined,
	};
}

const use = Object.fromEntries(
	Object.entries(staticRegistry.config.use).map(([name, definition]) => [
		name,
		dynamicActor({
			load: () => ({
				source: [
					`import { registry } from ${JSON.stringify(staticModulePath)};`,
					`export default registry.config.use[${JSON.stringify(name)}];`,
					"",
				].join("\n"),
			}),
			options: dynamicOptionsFor(definition),
		}),
	]),
);

export const registry = setup({ use });
