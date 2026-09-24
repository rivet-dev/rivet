import {
	type AgentSession,
	type CreateModelRuntimeOptions,
	ModelRuntime,
} from "@earendil-works/pi-coding-agent";
import { UserError } from "rivetkit";
import type { PiCredentialSource } from "./credentials.js";
import type { PiContext } from "./runtime.js";

/** Pi's credential store interface. `read`, `list`, `modify`, and `delete`, keyed by provider id. */
export type PiCredentialStore = NonNullable<CreateModelRuntimeOptions["credentials"]>;

/** A stored Pi credential: an API key or a subscription (OAuth) login. */
export type PiCredential = NonNullable<Awaited<ReturnType<PiCredentialStore["read"]>>>;

/** A custom provider, the same shape as one entry of Pi's `models.json` `providers`. */
export type PiProviderConfig = Parameters<ModelRuntime["registerProvider"]>[1];

/** A model from Pi's catalog. */
export type PiModel = NonNullable<ReturnType<ModelRuntime["getModel"]>>;

/** Model and credential options accepted by `pi()`. */
export interface PiModelOptions {
	/** The model for a new session, as `provider/modelId`, such as `anthropic/claude-opus-5-5`. */
	model?: string;
	/**
	 * Models a client may switch to with `setModel`, as `provider/modelId`.
	 * Without it, only `model`. Unlike Pi's scoped models, which only limit
	 * cycling, this is a hard limit.
	 */
	scopedModels?: string[];
	/** Custom providers registered on the actor's model runtime. */
	providers?: Record<string, PiProviderConfig>;
	/**
	 * API keys by provider id. They stay in the actor's memory and win over
	 * every other source.
	 */
	apiKeys?: Record<string, string>;
	/**
	 * Provider credentials the application manages, such as subscription
	 * logins. Called once per actor generation with the actor's context.
	 */
	credentials?: (c: PiContext) => PiCredentialSource;
}

/** A model a client may switch to. */
export interface PiModelInfo {
	provider: string;
	id: string;
	name: string;
	reasoning: boolean;
	input: PiModel["input"];
	contextWindow: number;
	maxTokens: number;
}

/** The model options that decide which models a client may use. */
type AllowlistOptions = Pick<PiModelOptions, "model" | "scopedModels">;

/**
 * Creates the model runtime for one actor generation. It never reads Pi's
 * `~/.pi/agent/auth.json` or `models.json`; credentials come from `apiKeys`,
 * then `credentials`, then the server environment.
 */
export async function createActorModelRuntime(
	options: Omit<PiModelOptions, "credentials">,
	credentials: PiCredentialStore,
): Promise<ModelRuntime> {
	const runtime = await ModelRuntime.create({ credentials, modelsPath: null });
	for (const [providerId, config] of Object.entries(options.providers ?? {})) {
		runtime.registerProvider(providerId, config);
	}
	for (const [providerId, apiKey] of Object.entries(options.apiKeys ?? {})) {
		await runtime.setRuntimeApiKey(providerId, apiKey);
	}
	await runtime.refresh({ allowNetwork: false });
	return runtime;
}

/** A credential store that holds nothing and refuses writes. */
export const emptyCredentialStore: PiCredentialStore = {
	read: async () => undefined,
	list: async () => [],
	modify: async (providerId, fn) => {
		const next = await fn(undefined);
		if (next !== undefined) {
			throw new Error(
				`pi actor has no credential storage for ${providerId}; configure pi({ credentials })`,
			);
		}
		return undefined;
	},
	delete: async () => {},
};

/** The models a client may switch to: `scopedModels`, or only `model`. */
function allowedModels(options: AllowlistOptions): string[] {
	return options.scopedModels ?? (options.model ? [options.model] : []);
}

function isAllowed(options: AllowlistOptions, provider: string, modelId: string): boolean {
	return allowedModels(options).includes(`${provider}/${modelId}`);
}

/** Looks up a `provider/modelId` in the actor's catalog. */
function catalogModel(runtime: ModelRuntime, name: string): PiModel {
	const slash = name.indexOf("/");
	const model = slash > 0 ? runtime.getModel(name.slice(0, slash), name.slice(slash + 1)) : undefined;
	if (!model) {
		throw new Error(`pi() model ${name} is not in Pi's model catalog; add it with providers`);
	}
	return model;
}

/**
 * The model a session opens with: the model saved in the session when it is
 * still allowed, otherwise the configured default. Undefined lets Pi choose,
 * which only happens when no model is configured.
 */
export function openingModel(
	options: AllowlistOptions,
	runtime: ModelRuntime,
	saved: { provider: string; modelId: string } | null,
): PiModel | undefined {
	if (saved && isAllowed(options, saved.provider, saved.modelId)) {
		const model = runtime.getModel(saved.provider, saved.modelId);
		if (model) return model;
	}
	const fallback = options.model ?? allowedModels(options)[0];
	return fallback ? catalogModel(runtime, fallback) : undefined;
}

/** Allowed models that have a credential, for a client's model picker. */
export function availableModels(
	options: AllowlistOptions,
	session: AgentSession,
): PiModelInfo[] {
	return session.modelRuntime
		.getAvailableSnapshot()
		.filter((model) => isAllowed(options, model.provider, model.id))
		.map((model) => ({
			provider: model.provider,
			id: model.id,
			name: model.name,
			reasoning: model.reasoning,
			input: model.input,
			contextWindow: model.contextWindow,
			maxTokens: model.maxTokens,
		}));
}

/** Throws `model_unavailable` when the session's model has no credential, for example after a logout. */
export function requireCredential(session: AgentSession): void {
	const model = session.model;
	if (!model) return;
	const available = session.modelRuntime
		.getAvailableSnapshot()
		.some((candidate) => candidate.provider === model.provider && candidate.id === model.id);
	if (!available) {
		throw new UserError(`No credential is configured for ${model.provider}/${model.id}.`, {
			code: "model_unavailable",
		});
	}
}

/**
 * Switches the session to an allowed model. The model object comes from the
 * runtime's catalog, so a client can never choose the URL a key is sent to.
 */
export async function switchModel(
	options: AllowlistOptions,
	session: AgentSession,
	provider: string,
	modelId: string,
): Promise<void> {
	if (!isAllowed(options, provider, modelId)) {
		throw new UserError(`Model ${provider}/${modelId} is not allowed for this agent.`, {
			code: "model_not_allowed",
		});
	}
	const model = session.modelRuntime
		.getAvailableSnapshot()
		.find((candidate) => candidate.provider === provider && candidate.id === modelId);
	if (!model) {
		throw new UserError(`No credential is configured for ${provider}/${modelId}.`, {
			code: "model_unavailable",
		});
	}
	await session.setModel(model);
}
