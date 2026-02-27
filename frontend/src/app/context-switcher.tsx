import { useClerk } from "@clerk/clerk-react";
import type { Rivet } from "@rivet-gg/cloud";
import {
	faChevronDown,
	faPlusCircle,
	faSlashForward,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
	useQuery,
} from "@tanstack/react-query";
import { useMatchRoute, useNavigate, useParams } from "@tanstack/react-router";
import { useState } from "react";
import {
	Button,
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	cn,
	Popover,
	PopoverContent,
	PopoverTrigger,
	Skeleton,
} from "@/components";
import {
	useCloudDataProvider,
	useEngineCompatDataProvider,
	useEngineDataProvider,
} from "@/components/actors";
import { SafeHover } from "@/components/safe-hover";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { LazyBillingPlanBadge } from "./billing/billing-plan-badge";

export function ContextSwitcher({ inline }: { inline?: boolean }) {
	const match = useContextSwitcherMatch();

	if (!match) {
		return null;
	}

	return (
		<ContextSwitcherInner
			inline={inline}
			organization={
				"organization" in match ? match.organization : undefined
			}
		/>
	);
}

function ContextSwitcherInner({
	organization,
	inline,
}: {
	organization?: string;
	inline?: boolean;
}) {
	const [isOpen, setIsOpen] = useState(false);

	if (__APP_TYPE__ === "cloud") {
		// biome-ignore lint/correctness/useHookAtTopLevel: guaranteed by build condition
		usePrefetchInfiniteQuery({
			// biome-ignore lint/correctness/useHookAtTopLevel: guaranteed by build condition
			...useCloudDataProvider().projectsQueryOptions({
				organization: organization!,
			}),
		});
	}

	return (
		<Popover open={isOpen} onOpenChange={setIsOpen}>
			<PopoverTrigger asChild>
				<Button
					variant={inline ? "ghost" : "outline"}
					className={cn(
						inline && "gap-2",
						"flex h-auto justify-between items-center px-2 py-1.5",
					)}
					endIcon={<Icon icon={faChevronDown} />}
				>
					<Breadcrumbs inline={inline} />
				</Button>
			</PopoverTrigger>
			<PopoverContent
				className="p-0 max-w-[calc(12rem*3)] w-full"
				align="start"
			>
				<Content onClose={() => setIsOpen(false)} />
			</PopoverContent>
		</Popover>
	);
}

const useContextSwitcherMatch = ():
	| {
			project: string;
			namespace: string;
			organization: string;
	  }
	| { organization: string; project: string }
	| { namespace: string }
	| false => {
	const match = useMatchRoute();

	const matchNamespace = match({
		to: "/orgs/$organization/projects/$project/ns/$namespace",
		fuzzy: true,
	});

	if (matchNamespace) {
		return matchNamespace;
	}

	const matchProject = match({
		to: "/orgs/$organization/projects/$project",
		fuzzy: true,
	});

	if (matchProject) {
		return matchProject;
	}

	const matchEngineNamespace = match({
		to: "/ns/$namespace",
		fuzzy: true,
	});

	if (matchEngineNamespace) {
		return matchEngineNamespace;
	}

	return false;
};

function Breadcrumbs({ inline }: { inline?: boolean }) {
	const match = useContextSwitcherMatch();

	if (match && "project" in match && "namespace" in match) {
		return (
			<div
				className={cn(
					"flex items-center min-w-0 w-full",
					inline && "flex-row justify-center gap-2",
					!inline && "flex-col",
				)}
			>
				<div
					className={cn(
						!inline && "text-xs min-w-0 w-full",
						"text-left text-muted-foreground flex",
					)}
				>
					<ProjectBreadcrumb
						project={match.project}
						className={cn(
							"truncate min-w-0 max-w-full block",
							inline && "h-auto",
							!inline && "h-4",
						)}
					/>
				</div>
				{inline ? <Icon icon={faSlashForward} /> : null}
				<div className="min-w-0 w-full">
					<NamespaceBreadcrumb
						className="text-left truncate block"
						namespace={match.namespace}
						project={match.project}
					/>
				</div>
			</div>
		);
	}

	if (match && "project" in match) {
		return <ProjectBreadcrumb project={match.project} />;
	}

	if (match && "namespace" in match) {
		return (
			<EngineNamespaceBreadcrumb
				className="text-left truncate block"
				namespace={match.namespace}
			/>
		);
	}

	return null;
}

function ProjectBreadcrumb({
	project,
	className,
}: {
	project: string;
	className?: string;
}) {
	const { isLoading, data } = useQuery(
		useCloudDataProvider().currentOrgProjectQueryOptions({ project }),
	);
	if (isLoading) {
		return <Skeleton className={cn("h-5 w-32", className)} />;
	}

	return (
		<span className={className}>
			{data?.displayName || "Unknown Project"}
		</span>
	);
}

function NamespaceBreadcrumb({
	namespace,
	project,
	className,
}: {
	namespace: string;
	project: string;
	className?: string;
}) {
	const { isLoading, data } = useQuery(
		useCloudDataProvider().currentOrgProjectNamespaceQueryOptions({
			project,
			namespace,
		}),
	);
	if (isLoading) {
		return <Skeleton className="h-5 w-32" />;
	}

	return (
		<span className={className}>
			{data?.displayName || "Unknown Namespace"}
		</span>
	);
}

function EngineNamespaceBreadcrumb({
	namespace,
	className,
}: {
	namespace: string;
	className?: string;
}) {
	const { isLoading, data } = useQuery(
		useEngineCompatDataProvider().namespaceQueryOptions(namespace),
	);
	if (isLoading) {
		return <Skeleton className={cn("h-5 w-32", className)} />;
	}

	return (
		<span className={className}>
			{data?.displayName || "Unknown Namespace"}
		</span>
	);
}

function Content({ onClose }: { onClose?: () => void }) {
	const params = useParams({
		strict: false,
		select: (p) => ({ organization: p.organization, project: p.project }),
	});

	const [currentProjectHover, setCurrentProjectHover] = useState<
		string | null
	>(params.project || null);

	if (!params.organization) {
		return;
	}

	return (
		<div className="flex w-full">
			<ProjectList
				organization={params.organization}
				onHover={setCurrentProjectHover}
				onClose={onClose}
			/>

			{currentProjectHover ? (
				<NamespaceList
					organization={params.organization}
					project={currentProjectHover}
					onClose={onClose}
				/>
			) : null}
		</div>
	);
}

function ProjectList({
	organization,
	onClose,
	onHover,
}: {
	organization: string;
	onClose?: () => void;
	onHover?: (project: string | null) => void;
}) {
	const { data, hasNextPage, isLoading, isFetchingNextPage, fetchNextPage } =
		useInfiniteQuery(
			useCloudDataProvider().projectsQueryOptions({
				organization: organization,
			}),
		);
	const navigate = useNavigate();
	const project = useParams({
		strict: false,
		select(params) {
			return params.project;
		},
	});
	const clerk = useClerk();

	return (
		<div className="border-l w-48">
			<Command loop>
				<CommandInput placeholder="Find project..." />
				<CommandList
					className="relative p-1 w-full"
					defaultValue={project}
				>
					<CommandGroup heading="Projects" className="w-full">
						{!isLoading ? (
							<CommandEmpty>
								No projects found.
								<Button
									className="mt-1"
									variant="outline"
									size="sm"
									startIcon={<Icon icon={faPlusCircle} />}
									onClick={() => {
										onHover?.(null);
										onClose?.();
										return navigate({
											to: ".",
											search: (old) => ({
												...old,
												modal: "create-project",
												organization,
											}),
										});
									}}
								>
									Create Project
								</Button>
							</CommandEmpty>
						) : null}

						{data
							?.sort((a, b) => {
								if (a.name === project) return -1;
								if (b.name === project) return 1;
								return 0;
							})
							.map((project, index) => {
								const Component =
									index < 5
										? PrefetchedProjectListItem
										: ProjectListItem;
								return (
									<Component
										key={project.id}
										{...project}
										onHover={() => onHover?.(project.name)}
										organization={organization}
										onSelect={() => {
											onClose?.();
											clerk.setActive({
												organization,
											});
											return navigate({
												to: "/orgs/$organization/projects/$project",
												params: {
													organization: organization,
													project: project.name,
												},
												search: {},
											});
										}}
									/>
								);
							})}
						{isLoading || isFetchingNextPage ? (
							<>
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
							</>
						) : null}

						<CommandItem
							keywords={["create", "new", "project"]}
							onSelect={() => {
								onHover?.(null);
								onClose?.();
								return navigate({
									to: ".",
									search: (old) => ({
										...old,
										modal: "create-project",
										organization,
									}),
								});
							}}
						>
							<Icon icon={faPlusCircle} className="mr-2" />
							Create Project
						</CommandItem>

						{hasNextPage ? (
							<VisibilitySensor onChange={fetchNextPage} />
						) : null}
					</CommandGroup>
				</CommandList>
			</Command>
		</div>
	);
}

function PrefetchedProjectListItem({
	id,
	name,
	displayName,
	...props
}: Rivet.Project & { organization: string }) {
	usePrefetchInfiniteQuery({
		...useCloudDataProvider().currentOrgProjectNamespacesQueryOptions({
			project: name,
		}),
	});

	return (
		<ProjectListItem
			id={id}
			name={name}
			displayName={displayName}
			{...props}
		/>
	);
}

function ProjectListItem({
	id,
	name,
	displayName,
	organization,
	onHover,
	onSelect,
}: Rivet.Project & {
	onHover?: () => void;
	onSelect?: () => void;
	organization: string;
}) {
	return (
		<SafeHover key={id} offset={40}>
			<CommandItem
				value={name}
				keywords={[displayName, name]}
				className="static w-full"
				onSelect={onSelect}
				onMouseEnter={onHover}
				onFocus={onHover}
			>
				<span className="truncate flex-1">{displayName}</span>
				<LazyBillingPlanBadge
					project={name}
					organization={organization}
				/>
			</CommandItem>
		</SafeHover>
	);
}

function ListItemSkeleton() {
	return (
		<div className="px-2 py-1.5">
			<Skeleton className="h-5 w-32" />
		</div>
	);
}

function NamespaceList({
	organization,
	project,
	onClose,
}: {
	organization: string;
	project: string;
	onClose?: () => void;
}) {
	const { data, hasNextPage, isLoading, isFetchingNextPage, fetchNextPage } =
		useInfiniteQuery(
			useCloudDataProvider().currentOrgProjectNamespacesQueryOptions({
				project,
			}),
		);
	const navigate = useNavigate();
	const clerk = useClerk();
	const namespace = useParams({
		strict: false,
		select(params) {
			return params.namespace;
		},
	});

	return (
		<div className="border-l w-48">
			<Command loop>
				<CommandInput placeholder="Find Namespace..." />
				<CommandList
					className="relative p-1 w-full"
					defaultValue={namespace}
				>
					<CommandGroup heading="Namespaces" className="w-full">
						{!isLoading ? (
							<CommandEmpty>
								No namespaces found.
								<Button
									className="mt-1"
									variant="outline"
									size="sm"
									startIcon={<Icon icon={faPlusCircle} />}
									onClick={() => {
										onClose?.();
										return navigate({
											to: ".",
											search: (old) => ({
												...old,
												modal: "create-ns",
											}),
										});
									}}
								>
									Create Namespace
								</Button>
							</CommandEmpty>
						) : null}

						{data
							?.sort((a, b) => {
								if (a.name === namespace) return -1;
								if (b.name === namespace) return 1;
								return 0;
							})
							.map((namespace) => (
								<SafeHover key={namespace.id} offset={40}>
									<CommandItem
										value={namespace.name}
										keywords={[
											namespace.displayName,
											namespace.name,
										]}
										className="static w-full"
										onSelect={() => {
											onClose?.();
											clerk.setActive({
												organization,
											});
											return navigate({
												to: "/orgs/$organization/projects/$project/ns/$namespace",
												params: {
													organization: organization,
													project: project,
													namespace: namespace.name,
												},
											});
										}}
									>
										<span className="truncate w-full">
											{namespace.displayName}
										</span>
									</CommandItem>
								</SafeHover>
							))}
						{isLoading || isFetchingNextPage ? (
							<>
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
							</>
						) : null}

						<CommandItem
							keywords={["create", "new", "namespace"]}
							onSelect={() => {
								onClose?.();
								return navigate({
									to: ".",
									search: (old) => ({
										...old,
										modal: "create-ns",
									}),
								});
							}}
						>
							<Icon icon={faPlusCircle} className="mr-2" />
							Create Namespace
						</CommandItem>

						{hasNextPage ? (
							<VisibilitySensor onChange={fetchNextPage} />
						) : null}
					</CommandGroup>
				</CommandList>
			</Command>
		</div>
	);
}
