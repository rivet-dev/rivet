import { useAuth } from "@/domains/auth/contexts/auth";
import { Skeleton, cn } from "@rivet-gg/components";
import { ErrorBoundary } from "@sentry/react";
import { useMatchRoute } from "@tanstack/react-router";
import { Suspense, useContext } from "react";
import { MobileBreadcrumbsContext } from "../breadcrumbs/mobile-breadcrumbs";
import { HeaderEnvironmentLinks } from "./links/header-environment-links";
import { HeaderGroupLinks } from "./links/header-group-links";
import { HeaderProjectLinks } from "./links/header-project-links";

function Content() {
	const matchRoute = useMatchRoute();

	const { profile } = useAuth();

	if (!profile?.identity.isRegistered) {
		return null;
	}

	const namespaceMatch = matchRoute({
		to: "/projects/$projectNameId/environments/$environmentNameId",
		fuzzy: true,
		pending: false,
	});

	if (namespaceMatch) {
		return (
			<HeaderEnvironmentLinks
				projectNameId={namespaceMatch.projectNameId}
				environmentNameId={namespaceMatch.environmentNameId}
			/>
		);
	}

	const projectMatch = matchRoute({
		to: "/projects/$projectNameId",
		fuzzy: true,
		pending: false,
	});

	if (projectMatch) {
		return (
			<HeaderProjectLinks projectNameId={projectMatch.projectNameId} />
		);
	}

	const groupMatch = matchRoute({
		to: "/teams/$groupId",
		fuzzy: true,
		pending: false,
	});

	if (groupMatch) {
		return <HeaderGroupLinks groupId={groupMatch.groupId} />;
	}

	return null;
}

export function HeaderSubNav() {
	const isMobile = useContext(MobileBreadcrumbsContext);

	return (
		<ErrorBoundary>
			<Suspense
				fallback={
					<div className="-mb-2 hidden md:flex min-h-10 items-center gap-6">
						<Skeleton className="mb-2 h-5 w-16" />
						<Skeleton className="mb-2 h-5 w-16" />
						<Skeleton className="mb-2 h-5 w-16" />
					</div>
				}
			>
				<div
					className={cn({
						"-mx-8 -mb-[9px] hidden md:flex min-h-10 items-center px-8 empty:hidden":
							!isMobile,
						"flex flex-col text-left pl-4 gap-6 border-l": isMobile,
					})}
				>
					<Content />
				</div>
			</Suspense>
		</ErrorBoundary>
	);
}
