import { GroupAvatar } from "@/domains/group/components/group-avatar";
import {
	environmentByIdQueryOptions,
	groupProjectsQueryOptions,
	projectByIdQueryOptions,
	projectEnvironmentDisplayNameQueryOptions,
	projectQueryOptions,
} from "@/domains/project/queries";
import { Badge, Skeleton } from "@rivet-gg/components";
import { Icon, faAngleRight, faPuzzle } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Fragment, Suspense } from "react";
import type { CommandPanelPage } from "./command-panel-navigation-provider";

function GroupBreadcrumbs({ groupId }: { groupId: string }) {
	const { data: group } = useSuspenseQuery(
		groupProjectsQueryOptions(groupId),
	);
	return (
		<>
			<GroupAvatar
				className="mr-2 size-4"
				displayName={group.displayName}
				avatarUrl={group.avatarUrl}
			/>
			{group.displayName}
		</>
	);
}

function ProjectBreadcrumb({ projectNameId }: { projectNameId: string }) {
	const {
		data: { gameId: projectId },
	} = useSuspenseQuery(projectByIdQueryOptions(projectNameId));
	const { data: project } = useSuspenseQuery(projectQueryOptions(projectId));

	return <>{project.displayName}</>;
}

function EnvironmentBreadcrumb({
	projectNameId,
	environmentNameId,
}: {
	projectNameId: string;
	environmentNameId: string;
}) {
	const {
		data: { gameId: projectId },
	} = useSuspenseQuery(projectByIdQueryOptions(projectNameId));

	const {
		data: { namespaceId: environmentId },
	} = useSuspenseQuery(
		environmentByIdQueryOptions({ projectId, environmentNameId }),
	);

	const { data: environment } = useSuspenseQuery(
		projectEnvironmentDisplayNameQueryOptions({ projectId, environmentId }),
	);
	return <span>{environment}</span>;
}

function BackendBreadcrumb() {
	return (
		<>
			<Icon icon={faPuzzle} className="mr-2 size-4" /> Backend
		</>
	);
}

interface CommandPanelNavigationBreadcrumbsProps {
	pages: CommandPanelPage[];
}

export function CommandPanelNavigationBreadcrumbs({
	pages,
}: CommandPanelNavigationBreadcrumbsProps) {
	if (pages.length === 0) {
		return null;
	}
	return (
		<div className="mt-2 flex min-h-8 items-center px-3">
			<Suspense fallback={<Skeleton className="h-4 w-10" />}>
				{pages.map((page, index) => (
					<Fragment key={page.key}>
						{index > 0 && (
							<Icon
								icon={faAngleRight}
								className="mr-2 size-3 text-muted-foreground"
							/>
						)}
						<Badge
							variant="outline"
							className="mr-2 flex items-center"
						>
							{page.key === "group" && (
								<GroupBreadcrumbs {...page.params} />
							)}
							{page.key === "project" && (
								<ProjectBreadcrumb {...page.params} />
							)}
							{page.key === "environment" && (
								<EnvironmentBreadcrumb {...page.params} />
							)}
							{page.key === "backend" && <BackendBreadcrumb />}
						</Badge>
					</Fragment>
				))}
			</Suspense>
		</div>
	);
}
