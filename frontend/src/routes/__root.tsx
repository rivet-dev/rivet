import type { QueryClient } from "@tanstack/react-query";
import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { match } from "ts-pattern";
import type {
	CloudContext,
	CloudNamespaceContext,
	EngineContext,
	EngineNamespaceContext,
	OrganizationContext,
	ProjectContext,
} from "@/app/data-providers/cache";
import { DevToolbar } from "@/app/dev-toolbar";
import { FullscreenLoading } from "@/components";

function RootRoute() {
	return (
		<>
			<Outlet />
			<DevToolbar />
			{import.meta.env.DEV ? (
				<TanStackRouterDevtools position="bottom-right" />
			) : null}
		</>
	);
}

function CloudRoute() {
	return (
		<>
			<Outlet />
			{import.meta.env.DEV ? (
				<TanStackRouterDevtools position="bottom-right" />
			) : null}
		</>
	);
}

interface RootRouteContext {
	queryClient: QueryClient;
	getOrCreateCloudContext: (...args: unknown[]) => CloudContext;
	getOrCreateEngineContext: (
		engineToken: (() => string) | string | (() => Promise<string>),
	) => EngineContext;
	getOrCreateOrganizationContext: (
		parent: CloudContext,
		organization: string,
	) => OrganizationContext;
	getOrCreateProjectContext: (
		parent: CloudContext & OrganizationContext,
		organization: string,
		project: string,
	) => ProjectContext;
	getOrCreateCloudNamespaceContext: (
		parent: CloudContext & OrganizationContext & ProjectContext,
		namespace: string,
		engineNamespaceName: string,
		engineNamespaceId: string,
	) => CloudNamespaceContext;
	getOrCreateEngineNamespaceContext: (
		parent: EngineContext,
		namespace: string,
	) => EngineNamespaceContext;
}

export const Route = createRootRouteWithContext<RootRouteContext>()({
	component: match(__APP_TYPE__)
		.with("cloud", () => CloudRoute)
		.otherwise(() => RootRoute),
	pendingComponent: FullscreenLoading,
});
