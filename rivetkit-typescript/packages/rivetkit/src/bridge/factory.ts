import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { getRunFunction, getRunInspectorConfig } from "@/actor/config";
import type {
	AnyActorDefinition,
	AnyStaticActorDefinition,
} from "@/actor/definition";
import { createClientWithDriver } from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import type { DynamicActorDefinition } from "@/dynamic/internal";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import type { RegistryConfig } from "@/registry/config";
import { buildActorConfig, sqliteBackendForConfig } from "@/registry/native";
import type {
	ActorFactoryHandle,
	CoreRuntime,
	RuntimeActorConfig,
} from "@/registry/runtime";
import {
	ensureDevChildBundle,
	moduleEntrySource,
	sourceEntrySource,
} from "./dev-bundle";
import {
	type BridgeSpawnPlan,
	buildBridgedFactory,
	buildBridgeRegistryConfig,
	runningFromSource,
} from "./host";

/**
 * Assemble bridged actor factories for the two bridge frontends: dynamic
 * actors (loader-resolved source) and worker-runtime static actors. Both
 * proxy the factory callback surface to a per-actor worker thread; this
 * module decides which callbacks exist, the RuntimeActorConfig the core sees,
 * and how the child bootstraps the definition.
 */

function assertBridgeSupported(runtime: CoreRuntime, actorName: string) {
	if (runtime.kind !== "napi") {
		throw new Error(
			`actor ${actorName} requires the native runtime; worker and dynamic actors are not supported on the ${runtime.kind} runtime`,
		);
	}
}

export function buildWorkerBridgedFactory(
	runtime: CoreRuntime,
	registryConfig: RegistryConfig,
	actorName: string,
	definition: AnyStaticActorDefinition,
): ActorFactoryHandle {
	assertBridgeSupported(runtime, actorName);
	const module = registryConfig.worker.module;
	if (!module) {
		throw new Error(
			`actor ${actorName} sets options.runtime: "worker" but the registry does not configure worker.module; pass import.meta.url from the module that exports the registry`,
		);
	}
	const exportName = registryConfig.worker.exportName;

	const resolveSpawn = async (): Promise<BridgeSpawnPlan> => {
		const bootstrap = {
			kind: "module",
			module,
			exportName,
		} as const;
		if (!runningFromSource()) {
			return { bootstrap };
		}
		const modulePath = module.startsWith("file:")
			? fileURLToPath(module)
			: module;
		const devBundlePath = await ensureDevChildBundle({
			cacheKey: `module:${modulePath}:${exportName ?? ""}`,
			entrySource: moduleEntrySource(modulePath),
		});
		return { bootstrap, devBundlePath };
	};

	return buildBridgedFactory(
		runtime,
		{
			resolveSpawn,
			registryConfig: buildBridgeRegistryConfig(registryConfig),
			actorName,
		},
		buildActorConfig(definition, registryConfig),
		{
			actionNames: Object.keys(
				(definition.config.actions ?? {}) as Record<string, unknown>,
			),
			useFallbackAction: false,
			callbackNames: computeCallbackNames(definition),
		},
	);
}

export function buildDynamicBridgedFactory(
	runtime: CoreRuntime,
	registryConfig: RegistryConfig,
	actorName: string,
	definition: DynamicActorDefinition,
): ActorFactoryHandle {
	assertBridgeSupported(runtime, actorName);
	const options = definition.options;

	const actorConfig: RuntimeActorConfig = {
		name: options.name,
		icon: options.icon,
		hasDatabase: options.database === true,
		remoteSqlite:
			options.database === true &&
			sqliteBackendForConfig(registryConfig) === "remote",
		// The loaded definition may declare state, so the core always loads
		// persist data for dynamic actors.
		hasState: true,
		canHibernateWebsocket: options.canHibernateWebSocket,
		actionTimeoutMs: options.actionTimeout,
		sleepTimeoutMs: options.sleepTimeout,
		sleepGracePeriodMs: options.sleepGracePeriod,
		noSleep: options.noSleep,
		maxQueueSize: options.maxQueueSize,
		maxQueueMessageSize: options.maxQueueMessageSize,
		maxIncomingMessageSize: registryConfig.maxIncomingMessageSize,
		maxOutgoingMessageSize: registryConfig.maxOutgoingMessageSize,
		actions: [],
	};

	const client = () =>
		Promise.resolve(
			createClientWithDriver(
				new RemoteEngineControlClient(
					convertRegistryConfigToClientConfig(registryConfig),
				),
				{ encoding: "bare" },
			),
		);

	const resolveSpawn = async ({
		key,
	}: {
		key: string[];
	}): Promise<BridgeSpawnPlan> => {
		const result = await definition.loader({ key, client });
		const sourcePath = await writeDynamicSource(
			result.source,
			result.sourceFormat ?? "esm-js",
		);
		const workerResourceLimits =
			result.worker?.memoryLimitMb !== undefined
				? { maxOldGenerationSizeMb: result.worker.memoryLimitMb }
				: undefined;
		const bootstrap = {
			kind: "source",
			sourcePath,
			workerResourceLimits,
		} as const;
		if (!runningFromSource()) {
			return { bootstrap };
		}
		const devBundlePath = await ensureDevChildBundle({
			cacheKey: `source:${sourcePath}`,
			entrySource: sourceEntrySource(sourcePath),
		});
		return { bootstrap, devBundlePath };
	};

	return buildBridgedFactory(
		runtime,
		{
			resolveSpawn,
			registryConfig: buildBridgeRegistryConfig(registryConfig),
			actorName,
		},
		actorConfig,
		{
			// The loaded definition's action names are unknown at factory
			// creation, so dispatch routes through the fallback action and the
			// full callback surface is registered.
			useFallbackAction: true,
		},
	);
}

/**
 * Write loader-resolved source under a content-hash directory so identical
 * sources share one file (and one dev bundle). Files persist for the process
 * lifetime; the directory lives under the working directory so module
 * resolution of bare specifiers in the source walks up into the application's
 * node_modules.
 */
async function writeDynamicSource(
	source: string,
	format: "esm-js" | "commonjs-js",
): Promise<string> {
	const hash = createHash("sha256").update(source).digest("hex").slice(0, 16);
	const dir = path.join(process.cwd(), ".rivetkit", "dynamic-actors", hash);
	await mkdir(dir, { recursive: true });
	const fileName =
		format === "esm-js" ? "actor-source.mjs" : "actor-source.cjs";
	const filePath = path.join(dir, fileName);
	await writeFile(filePath, source);
	return filePath;
}

/**
 * Mirror of the registration conditions in buildNativeFactory: a bridged
 * worker actor must register exactly the callbacks the in-process bag would,
 * because callback presence changes core behavior (connection state
 * machinery, disconnect bookkeeping, run-handler lifecycle).
 */
function computeCallbackNames(definition: AnyActorDefinition): string[] {
	const config = definition.config as Record<string, any>;
	const names = [
		"onSleep",
		"onDestroy",
		"onRequest",
		"onQueueSend",
		"serializeState",
	];
	const hasStaticState = "state" in config;
	const hasStaticConnState = Object.hasOwn(config, "connState");
	const hasDynamicConnState = typeof config.createConnState === "function";
	if (hasStaticState || typeof config.createState === "function") {
		names.push("createState");
	}
	if (typeof config.onCreate === "function") {
		names.push("onCreate");
	}
	if ("vars" in config || typeof config.createVars === "function") {
		names.push("createVars");
	}
	if (typeof config.onMigrate === "function" || config.db !== undefined) {
		names.push("onMigrate");
	}
	if (typeof config.onWake === "function") {
		names.push("onWake");
	}
	if (typeof config.onBeforeActorStart === "function") {
		names.push("onBeforeActorStart");
	}
	if (typeof config.onBeforeConnect === "function") {
		names.push("onBeforeConnect");
	}
	if (hasStaticConnState || hasDynamicConnState) {
		names.push("createConnState");
	}
	if (typeof config.onConnect === "function") {
		names.push("onConnect");
	}
	if (
		typeof config.onDisconnect === "function" ||
		hasStaticConnState ||
		hasDynamicConnState ||
		config.options?.canHibernateWebSocket === true
	) {
		names.push("onDisconnectFinal");
	}
	if (
		config.events &&
		Object.values(config.events as Record<string, unknown>).some(
			(schema) =>
				typeof (schema as { canSubscribe?: unknown }).canSubscribe ===
				"function",
		)
	) {
		names.push("onBeforeSubscribe");
	}
	if (typeof config.onBeforeActionResponse === "function") {
		names.push("onBeforeActionResponse");
	}
	if (typeof config.onWebSocket === "function") {
		names.push("onWebSocket");
	}
	if (getRunFunction(config.run)) {
		names.push("run");
	}
	if (getRunInspectorConfig(config.run) !== undefined) {
		names.push("getWorkflowHistory", "replayWorkflow");
	}
	return names;
}
