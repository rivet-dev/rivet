import type { Clerk } from "@clerk/clerk-js";
import {
	createNamespaceContext as createCloudNamespaceContext,
	createGlobalContext as createGlobalCloudContext,
	createOrganizationContext,
	createProjectContext,
} from "@/app/data-providers/cloud-data-provider";
import {
	createNamespaceContext as createEngineNamespaceContext,
	createGlobalContext as createGlobalEngineContext,
} from "@/app/data-providers/engine-data-provider";
import { createGlobalContext as createGlobalInspectorContext } from "@/app/data-providers/inspector-data-provider";

// Cache factories for data providers to maintain stable references across navigation
export type CloudContext = ReturnType<typeof createGlobalCloudContext>;
export type CloudNamespaceContext = ReturnType<
	typeof createCloudNamespaceContext
>;
export type EngineContext = ReturnType<typeof createGlobalEngineContext>;
export type EngineNamespaceContext = ReturnType<
	typeof createEngineNamespaceContext
>;
export type InspectorContext = ReturnType<typeof createGlobalInspectorContext>;
export type OrganizationContext = ReturnType<typeof createOrganizationContext>;
export type ProjectContext = ReturnType<typeof createProjectContext>;

let cloudContextCache: CloudContext | null = null;
const cloudNamespaceContextCache = new Map<string, CloudNamespaceContext>();
const engineContextCache = new Map<string, EngineContext>();
const engineNamespaceContextCache = new Map<string, EngineNamespaceContext>();
const inspectorContextCache = new Map<string, InspectorContext>();
const organizationContextCache = new Map<string, OrganizationContext>();
const projectContextCache = new Map<string, ProjectContext>();

export function getOrCreateCloudContext(clerk: Clerk): CloudContext {
	if (!cloudContextCache) {
		cloudContextCache = createGlobalCloudContext({ clerk });
	}
	return cloudContextCache;
}

export function getOrCreateEngineContext(
	engineToken: (() => string) | string | (() => Promise<string>),
): EngineContext {
	const key =
		typeof engineToken === "function"
			? engineToken.toString()
			: engineToken;
	const cached = engineContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createGlobalEngineContext({ engineToken });
	engineContextCache.set(key, context);
	return context;
}

export function getOrCreateInspectorContext(opts: {
	url?: string;
	token?: string;
}): InspectorContext {
	const key = `${opts.url ?? ""}:${opts.token ?? ""}`;
	const cached = inspectorContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createGlobalInspectorContext(opts);
	inspectorContextCache.set(key, context);
	return context;
}

export function getOrCreateOrganizationContext(
	parent: CloudContext,
	organization: string,
): OrganizationContext {
	const key = organization;
	const cached = organizationContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createOrganizationContext({
		...parent,
		organization,
	});
	organizationContextCache.set(key, context);
	return context;
}

export function getOrCreateProjectContext(
	parent: CloudContext & OrganizationContext,
	organization: string,
	project: string,
): ProjectContext {
	const key = `${organization}:${project}`;
	const cached = projectContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createProjectContext({
		...parent,
		organization,
		project,
	});
	projectContextCache.set(key, context);
	return context;
}

export function getOrCreateCloudNamespaceContext(
	parent: CloudContext & OrganizationContext & ProjectContext,
	namespace: string,
	engineNamespaceName: string,
	engineNamespaceId: string,
): CloudNamespaceContext {
	const key = `${parent.organization}:${parent.project}:${namespace}`;
	const cached = cloudNamespaceContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createCloudNamespaceContext({
		...parent,
		namespace,
		engineNamespaceName,
		engineNamespaceId,
	});
	cloudNamespaceContextCache.set(key, context);
	return context;
}

export function getOrCreateEngineNamespaceContext(
	parent: EngineContext,
	namespace: string,
): EngineNamespaceContext {
	const key = `${parent.engineToken}:${namespace}`;
	const cached = engineNamespaceContextCache.get(key);
	if (cached) {
		return cached;
	}
	const context = createEngineNamespaceContext({
		...parent,
		namespace,
	});
	engineNamespaceContextCache.set(key, context);
	return context;
}
