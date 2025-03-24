import { useAuth } from "@/domains/auth/contexts/auth";

import { Skeleton, cn } from "@rivet-gg/components";
import { ErrorBoundary } from "@sentry/react";
import { useMatchRoute, useRouterState } from "@tanstack/react-router";
import { Suspense, useContext } from "react";
import { EnvironmentBreadcrumb } from "./environment-breadcrumb";
import { GroupBreadcrumb } from "./group-breadcrumb";
import { MobileBreadcrumbsContext } from "./mobile-breadcrumbs";
import { ProjectBreadcrumb } from "./project-breadcrumb";

function Content() {
	const matchRoute = useMatchRoute();

	const { profile } = useAuth();
	if (!profile?.identity.isRegistered) {
		return null;
	}

	const groupMatch = matchRoute({ to: "/teams/$groupId", fuzzy: true });

	if (groupMatch) {
		return <GroupBreadcrumb groupId={groupMatch.groupId} />;
	}

	const projectEnvMatch = matchRoute({
		to: "/projects/$projectNameId/environments/$environmentNameId",
		fuzzy: true,
	});

	if (projectEnvMatch) {
		return (
			<EnvironmentBreadcrumb
				environmentNameId={projectEnvMatch.environmentNameId}
				projectNameId={projectEnvMatch.projectNameId}
			/>
		);
	}

	const projectMatch = matchRoute({
		to: "/projects/$projectNameId",
		fuzzy: true,
	});

	if (projectMatch) {
		return <ProjectBreadcrumb projectNameId={projectMatch.projectNameId} />;
	}

	return null;
}

export function Breadcrumbs() {
	const isMobile = useContext(MobileBreadcrumbsContext);
	const pathname = useRouterState({
		select: (state) => state.location.pathname,
	});
	return (
		<div
			className={cn(
				"flex",
				isMobile && "flex-col gap-6",
				!isMobile && "items-center gap-2",
			)}
		>
			<ErrorBoundary
				key={pathname}
				onError={(...args) => {
					console.log(
						"Error occurred while rendering breadcrumbs.",
						...args,
					);
				}}
			>
				<Suspense
					fallback={
						<>
							<Skeleton className="h-5 w-16" />
							<Skeleton className="h-5 w-16" />
							<Skeleton className="h-5 w-16" />
						</>
					}
				>
					<Content />
				</Suspense>
			</ErrorBoundary>
		</div>
	);
}
