import type { Rivet } from "@rivet-gg/cloud";
import {
	faChevronDown,
	faCheck,
	faGear,
	faPlus,
	faPlusCircle,
	faSlashForward,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
	useQuery,
} from "@tanstack/react-query";
import {
	useMatches,
	useMatchRoute,
	useNavigate,
	useParams,
	useSearch,
} from "@tanstack/react-router";
import { useState } from "react";
import {
	RECENT_NAMESPACES_KEY,
	RECENT_PROJECTS_KEY,
	getRecentTimestamp,
} from "@/lib/recently-visited";
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
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";
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

	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guaranteed by build condition
		usePrefetchInfiniteQuery({
			// biome-ignore lint/correctness/useHookAtTopLevel: guaranteed by build condition
			...useCloudDataProvider().projectsQueryOptions({
				organization: organization!,
			}),
		});
	}

	// biome-ignore lint/correctness/useHookAtTopLevel: usage is stable inside this function
	const match = useContextSwitcherMatch();

	// Multitenancy inline case: render per-segment popovers so each chevron
	// opens its own dropdown (project / namespace), matching the v77 + v78
	// mockups. Other cases fall back to the legacy single popover below.
	if (
		inline &&
		match &&
		"project" in match &&
		"namespace" in match &&
		"organization" in match
	) {
		return (
			<div className="flex items-center min-w-0">
				<ProjectSegmentPopover
					organization={match.organization}
					currentProject={match.project}
				/>
				<Icon
					icon={faSlashForward}
					className="text-muted-foreground/40 mx-1 shrink-0"
				/>
				<NamespaceSegmentPopover
					organization={match.organization}
					currentProject={match.project}
					currentNamespace={match.namespace}
				/>
				<ActorBreadcrumbSegment />
			</div>
		);
	}

	// Project-only landing (e.g. /orgs/$org/projects/$project namespaces grid).
	// Render just the project segment with its own dropdown — the legacy
	// 2-column popover doesn't fit here.
	if (
		inline &&
		match &&
		"project" in match &&
		"organization" in match &&
		!("namespace" in match)
	) {
		return (
			<div className="flex items-center min-w-0">
				<ProjectSegmentPopover
					organization={match.organization}
					currentProject={match.project}
				/>
			</div>
		);
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
				className="p-0 w-fit max-w-[calc(12rem*3)]"
				align="start"
			>
				<Content onClose={() => setIsOpen(false)} />
			</PopoverContent>
		</Popover>
	);
}

function ProjectSegmentPopover({
	organization,
	currentProject,
}: {
	organization: string;
	currentProject: string;
}) {
	const [open, setOpen] = useState(false);
	const { data: projectData } = useQuery(
		useCloudDataProvider().currentOrgProjectQueryOptions({
			project: currentProject,
		}),
	);
	const label = projectData?.displayName ?? currentProject;

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					className="flex h-auto items-center gap-1.5 px-2 py-1 text-sm font-medium text-foreground hover:bg-foreground/[0.06]"
					endIcon={
						<Icon
							icon={faChevronDown}
							className="size-2.5 opacity-60"
						/>
					}
				>
					<span className="truncate">{label}</span>
				</Button>
			</PopoverTrigger>
			<PopoverContent className="p-0 w-56" align="start">
				<ProjectList
					organization={organization}
					currentProject={currentProject}
					onClose={() => setOpen(false)}
				/>
			</PopoverContent>
		</Popover>
	);
}

function NamespaceSegmentPopover({
	organization,
	currentProject,
	currentNamespace,
}: {
	organization: string;
	currentProject: string;
	currentNamespace: string;
}) {
	const [open, setOpen] = useState(false);
	const [hoveredNamespace, setHoveredNamespace] = useState<string | null>(
		currentNamespace,
	);
	const { data: nsData } = useQuery(
		useCloudDataProvider().currentOrgProjectNamespaceQueryOptions({
			project: currentProject,
			namespace: currentNamespace,
		}),
	);
	const label = nsData?.displayName ?? currentNamespace;

	return (
		<Popover
			open={open}
			onOpenChange={(next) => {
				setOpen(next);
				if (next) {
					setHoveredNamespace(currentNamespace);
				}
			}}
		>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					className="flex h-auto items-center gap-1.5 px-2 py-1 text-sm font-medium text-foreground hover:bg-foreground/[0.06]"
					endIcon={
						<Icon
							icon={faChevronDown}
							className="size-2.5 opacity-60"
						/>
					}
				>
					<span className="truncate">{label}</span>
				</Button>
			</PopoverTrigger>
			<PopoverContent className="p-0 w-fit flex" align="start">
				<div className="w-56">
					<NamespaceList
						organization={organization}
						project={currentProject}
						currentNamespace={currentNamespace}
						onHover={setHoveredNamespace}
						onClose={() => setOpen(false)}
					/>
				</div>
				{hoveredNamespace ? (
					<ActorsList
						organization={organization}
						project={currentProject}
						namespace={hoveredNamespace}
						onClose={() => setOpen(false)}
					/>
				) : null}
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
					"flex items-center min-w-0",
					inline && "flex-row gap-2 max-w-full",
					!inline && "flex-col w-full",
				)}
			>
				<div
					className={cn(
						!inline && "text-xs min-w-0 w-full",
						"text-left text-muted-foreground flex",
						inline && "shrink-0",
					)}
				>
					<ProjectBreadcrumb
						project={match.project}
						className={cn(
							inline ? "whitespace-nowrap" : "truncate min-w-0 max-w-full block",
							inline && "h-auto",
							!inline && "h-4",
						)}
					/>
				</div>
				{inline ? <Icon icon={faSlashForward} className="shrink-0" /> : null}
				<div className={cn(!inline && "min-w-0 w-full", inline && "shrink-0")}>
					<NamespaceBreadcrumb
						className={cn(
							"text-left block",
							inline ? "whitespace-nowrap" : "truncate",
						)}
						namespace={match.namespace}
						project={match.project}
					/>
				</div>
				{inline ? <ActorBreadcrumbSegment /> : null}
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

function ActorBreadcrumbSegment() {
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by the parent only rendering on namespace match
	const search = useSearch({ strict: false }) as { n?: string[] };
	const buildId = search.n?.[0];

	if (!buildId) return null;

	return (
		<>
			<Icon
				icon={faSlashForward}
				className="text-muted-foreground/40 mx-1 shrink-0"
			/>
			<ActorSegmentPopover currentBuildId={buildId} />
		</>
	);
}

function ActorSegmentPopover({ currentBuildId }: { currentBuildId: string }) {
	const [open, setOpen] = useState(false);
	const navigate = useNavigate();
	const { data: builds = [] } = useInfiniteQuery(
		useEngineCompatDataProvider().buildsQueryOptions(),
	);

	const currentBuild = builds.find((b) => b.id === currentBuildId);
	const currentMeta = currentBuild?.name?.metadata as
		| Record<string, unknown>
		| undefined;
	const currentLabel =
		typeof currentMeta?.name === "string" ? currentMeta.name : currentBuildId;

	const sorted = [...builds].sort((a, b) => {
		const an =
			(a.name?.metadata as Record<string, unknown> | undefined)?.name;
		const bn =
			(b.name?.metadata as Record<string, unknown> | undefined)?.name;
		const aLabel = typeof an === "string" ? an : a.id;
		const bLabel = typeof bn === "string" ? bn : b.id;
		return aLabel.localeCompare(bLabel);
	});

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					className="flex h-auto items-center gap-1.5 px-2 py-1 text-sm font-medium text-foreground hover:bg-foreground/[0.06]"
					endIcon={
						<Icon
							icon={faChevronDown}
							className="size-2.5 opacity-60"
						/>
					}
				>
					<span className="truncate">{currentLabel}</span>
				</Button>
			</PopoverTrigger>
			<PopoverContent className="p-0 w-56" align="start">
				<div className="w-full">
					<Command loop>
						<CommandInput placeholder="Find actor..." />
						<CommandList
							className="relative p-1 w-full"
							defaultValue={currentBuildId}
						>
							<CommandGroup heading="Actors" className="w-full">
								{sorted.length === 0 ? (
									<CommandEmpty>No actors yet.</CommandEmpty>
								) : null}
								{sorted.map((build) => {
									const meta = build.name?.metadata as
										| Record<string, unknown>
										| undefined;
									const label =
										typeof meta?.name === "string"
											? meta.name
											: build.id;
									const isCurrent = build.id === currentBuildId;
									return (
										<CommandItem
											key={build.id}
											value={build.id}
											keywords={[label, build.id]}
											className="static w-full"
											onSelect={() => {
												setOpen(false);
												return navigate({
													to: ".",
													search: (old) => ({
														...(old as Record<
															string,
															unknown
														>),
														actorId: undefined,
														actorKey: undefined,
														n: [build.id],
													}),
												});
											}}
										>
											<Icon
												icon={faCheck}
												className={cn(
													"mr-2 size-3 shrink-0 text-primary",
													isCurrent
														? "opacity-100"
														: "opacity-0",
												)}
											/>
											<span className="truncate w-full">
												{label}
											</span>
										</CommandItem>
									);
								})}
								<CommandItem
									keywords={["create", "new", "actor"]}
									className="text-primary"
									onSelect={() => {
										setOpen(false);
										return navigate({
											to: ".",
											search: (old) => ({
												...(old as Record<string, unknown>),
												modal: "create-actor",
											}),
										});
									}}
								>
									<Icon
										icon={faPlus}
										className="mr-2 size-3 text-primary"
									/>
									New Actor
								</CommandItem>
							</CommandGroup>
						</CommandList>
					</Command>
				</div>
			</PopoverContent>
		</Popover>
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
		select: (p) => ({
			organization: p.organization,
			project: p.project,
			namespace: p.namespace,
		}),
	});

	const [currentProjectHover, setCurrentProjectHover] = useState<
		string | null
	>(params.project || null);
	const [currentNamespaceHover, setCurrentNamespaceHover] = useState<
		string | null
	>(params.namespace || null);

	if (!params.organization) {
		return;
	}

	return (
		<div className="flex w-full">
			<ProjectList
				organization={params.organization}
				onHover={(next) => {
					setCurrentProjectHover(next);
					// Reset namespace hover when project changes so we don't
					// render a stale Actors column.
					if (next !== currentProjectHover) {
						setCurrentNamespaceHover(null);
					}
				}}
				onClose={onClose}
			/>

			{currentProjectHover ? (
				<NamespaceList
					organization={params.organization}
					project={currentProjectHover}
					onHover={setCurrentNamespaceHover}
					onClose={onClose}
				/>
			) : null}

			{currentProjectHover && currentNamespaceHover ? (
				<ActorsList
					organization={params.organization}
					project={currentProjectHover}
					namespace={currentNamespaceHover}
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
	currentProject,
}: {
	organization: string;
	onClose?: () => void;
	onHover?: (project: string | null) => void;
	currentProject?: string;
}) {
	const { data, hasNextPage, isLoading, isFetchingNextPage, fetchNextPage } =
		useInfiniteQuery(
			useCloudDataProvider().projectsQueryOptions({
				organization: organization,
			}),
		);
	const navigate = useNavigate();
	const paramsProject = useParams({
		strict: false,
		select(params) {
			return params.project;
		},
	});
	const project = currentProject ?? paramsProject;

	return (
		<div className="w-full">
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
									startIcon={<Icon icon={faPlus} />}
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
									New Project
								</Button>
							</CommandEmpty>
						) : null}

						{data
							?.sort((a, b) => {
								const aTime = getRecentTimestamp(RECENT_PROJECTS_KEY, a.name);
								const bTime = getRecentTimestamp(RECENT_PROJECTS_KEY, b.name);
								return bTime - aTime;
							})
							.map((p, index) => {
								const Component =
									index < 5
										? PrefetchedProjectListItem
										: ProjectListItem;
								return (
									<Component
										key={p.id}
										{...p}
										isCurrent={project === p.name}
										onHover={() => onHover?.(p.name)}
										organization={organization}
										onClose={onClose}
										onSelect={() => {
											onClose?.();
											authClient.organization.setActive({
												organizationSlug: organization,
											});
											return navigate({
												to: "/orgs/$organization/projects/$project",
												params: {
													organization: organization,
													project: p.name,
												},
												search: (old) => ({ ...old }),
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
							className="text-primary"
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
							<Icon icon={faPlus} className="mr-2 size-3 text-primary" />
							New Project
						</CommandItem>

						{hasNextPage && !isFetchingNextPage ? (
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
}: Rivet.Project & {
	organization: string;
	isCurrent?: boolean;
	onHover?: () => void;
	onSelect?: () => void;
	onClose?: () => void;
}) {
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
	isCurrent,
	onHover,
	onSelect,
	onClose,
}: Rivet.Project & {
	onHover?: () => void;
	onSelect?: () => void;
	onClose?: () => void;
	organization: string;
	isCurrent?: boolean;
}) {
	const navigate = useNavigate();
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
				<Icon
					icon={faCheck}
					className={cn(
						"mr-2 size-3 shrink-0 text-primary",
						isCurrent ? "opacity-100" : "opacity-0",
					)}
				/>
				<span className="truncate flex-1">{displayName}</span>
				{features.billing && (
					<button
						type="button"
						aria-label={`Billing for ${displayName}`}
						title="Manage billing"
						onPointerDown={(e) => e.stopPropagation()}
						onClick={(e) => {
							e.stopPropagation();
							e.preventDefault();
							onClose?.();
							authClient.organization.setActive({
								organizationSlug: organization,
							});
							void navigate({
								to: "/orgs/$organization/projects/$project",
								params: { organization, project: name },
								search: { settings: "billing" },
							});
						}}
						// `relative z-10` lifts the badge above SafeHover's
						// click-eating `::before` corridor, the same trick the
						// gear icon uses below.
						className="relative z-10 rounded-full focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
					>
						<LazyBillingPlanBadge
							project={name}
							organization={organization}
						/>
					</button>
				)}
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
	onHover,
	currentNamespace,
}: {
	organization: string;
	project: string;
	onClose?: () => void;
	onHover?: (namespace: string | null) => void;
	currentNamespace?: string;
}) {
	const { data, hasNextPage, isLoading, isFetchingNextPage, fetchNextPage } =
		useInfiniteQuery(
			useCloudDataProvider().currentOrgProjectNamespacesQueryOptions({
				project,
			}),
		);
	const navigate = useNavigate();
	const paramsNamespace = useParams({
		strict: false,
		select(params) {
			return params.namespace;
		},
	});
	const leafFullPath = useMatches({
		select: (matches) => matches[matches.length - 1]?.fullPath,
	});
	const namespaceBase =
		"/orgs/$organization/projects/$project/ns/$namespace";
	const namespaceTo = (
		typeof leafFullPath === "string" &&
		leafFullPath.startsWith(namespaceBase)
			? leafFullPath
			: namespaceBase
	) as "/orgs/$organization/projects/$project/ns/$namespace";
	const namespace = currentNamespace ?? paramsNamespace;

	return (
		<div className="w-full">
			<Command loop>
				<CommandInput placeholder="Find namespace..." />
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
									startIcon={<Icon icon={faPlus} />}
									onClick={() => {
										onClose?.();
										return navigate({
											to: ".",
											search: (old) => ({
												...old,
												modal: "create-ns",
												project: project,
											}),
										});
									}}
								>
									New Namespace
								</Button>
							</CommandEmpty>
						) : null}

						{data
							?.sort((a, b) => {
								const aTime = getRecentTimestamp(RECENT_NAMESPACES_KEY, a.name);
								const bTime = getRecentTimestamp(RECENT_NAMESPACES_KEY, b.name);
								return bTime - aTime;
							})
							.map((ns) => {
								const isCurrent = ns.name === namespace;
								return (
									<SafeHover key={ns.id} offset={40}>
										<CommandItem
											value={ns.name}
											keywords={[ns.displayName, ns.name]}
											className="group static w-full"
											onMouseEnter={() => onHover?.(ns.name)}
											onFocus={() => onHover?.(ns.name)}
											onSelect={() => {
												onClose?.();
												authClient.organization.setActive({
													organizationSlug: organization,
												});
												return navigate({
													to: namespaceTo,
													params: {
														organization: organization,
														project: project,
														namespace: ns.name,
													},
													search: (old) => ({ ...old }),
												});
											}}
										>
											<Icon
												icon={faCheck}
												className={cn(
													"mr-2 size-3 shrink-0 text-primary",
													isCurrent
														? "opacity-100"
														: "opacity-0",
												)}
											/>
											<span className="truncate flex-1">
												{ns.displayName}
											</span>
											<button
												type="button"
												aria-label={`Settings for ${ns.displayName}`}
												title="Namespace settings"
												onPointerDown={(e) => {
													// Stop cmdk's onSelect from firing on the
													// parent CommandItem so the gear is its
													// own navigation, not a row click.
													e.stopPropagation();
												}}
												onClick={(e) => {
													e.stopPropagation();
													e.preventDefault();
													onClose?.();
													authClient.organization.setActive({
														organizationSlug: organization,
													});
													void navigate({
														to: "/orgs/$organization/projects/$project/ns/$namespace",
														params: {
															organization,
															project,
															namespace: ns.name,
														},
														search: { settings: "settings" },
													});
												}}
												// `relative z-10` is load-bearing: the SafeHover
												// parent paints a click-eating `::before` corridor
												// at `z-index: 1` that overlaps this column.
												// Without lifting the button above it, the
												// gear is unclickable on hover.
												className={cn(
													"relative z-10 ml-2 -my-1 size-6 rounded inline-flex items-center justify-center shrink-0",
													"text-muted-foreground hover:text-foreground hover:bg-foreground/[0.08]",
													"opacity-0 transition-opacity",
													"group-hover:opacity-100 group-data-[selected=true]:opacity-100 focus-visible:opacity-100",
													"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
												)}
											>
												<Icon icon={faGear} className="size-3" />
											</button>
										</CommandItem>
									</SafeHover>
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
							keywords={["create", "new", "namespace"]}
							className="text-primary"
							onSelect={() => {
								onClose?.();
								return navigate({
									to: ".",
									search: (old) => ({
										...old,
										modal: "create-ns",
										project: project,
									}),
								});
							}}
						>
							<Icon icon={faPlus} className="mr-2 size-3 text-primary" />
							New Namespace
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

function ActorsList({
	organization,
	project,
	namespace,
	onClose,
}: {
	organization: string;
	project: string;
	namespace: string;
	onClose?: () => void;
}) {
	const navigate = useNavigate();
	const {
		data: actors,
		isLoading,
		hasNextPage,
		isFetchingNextPage,
		fetchNextPage,
	} = useInfiniteQuery(
		useCloudDataProvider().currentOrgProjectNamespaceActorNamesQueryOptions({
			project,
			namespace,
		}),
	);

	const sorted = [...(actors ?? [])].sort((a, b) => {
		const an = (a.name?.metadata as Record<string, unknown> | undefined)
			?.name;
		const bn = (b.name?.metadata as Record<string, unknown> | undefined)
			?.name;
		const aLabel = typeof an === "string" ? an : a.id;
		const bLabel = typeof bn === "string" ? bn : b.id;
		return aLabel.localeCompare(bLabel);
	});

	return (
		<div className="border-l w-48">
			<Command loop>
				<CommandInput placeholder="Find Actor..." />
				<CommandList className="relative p-1 w-full">
					<CommandGroup heading="Actors" className="w-full">
						{!isLoading && sorted.length === 0 ? (
							<CommandEmpty>No actors yet.</CommandEmpty>
						) : null}
						{isLoading ? (
							<>
								<ListItemSkeleton />
								<ListItemSkeleton />
								<ListItemSkeleton />
							</>
						) : null}
						{sorted.map((actor) => {
							const meta = actor.name?.metadata as
								| Record<string, unknown>
								| undefined;
							const label =
								typeof meta?.name === "string"
									? meta.name
									: actor.id;
							return (
								<CommandItem
									key={actor.id}
									value={actor.id}
									keywords={[label, actor.id]}
									className="static w-full"
									onSelect={() => {
										onClose?.();
										authClient.organization.setActive({
											organizationSlug: organization,
										});
										return navigate({
											to: "/orgs/$organization/projects/$project/ns/$namespace",
											params: {
												organization,
												project,
												namespace,
											},
											search: (old) => ({
												...(old as Record<string, unknown>),
												actorId: undefined,
												actorKey: undefined,
												n: [actor.id],
											}),
										});
									}}
								>
									<span className="truncate w-full">{label}</span>
								</CommandItem>
							);
						})}
						{isFetchingNextPage ? (
							<>
								<ListItemSkeleton />
								<ListItemSkeleton />
							</>
						) : null}
						{hasNextPage && !isFetchingNextPage ? (
							<VisibilitySensor onChange={fetchNextPage} />
						) : null}

						<CommandItem
							keywords={["create", "new", "actor"]}
							onSelect={() => {
								onClose?.();
								authClient.organization.setActive({
									organizationSlug: organization,
								});
								return navigate({
									to: "/orgs/$organization/projects/$project/ns/$namespace",
									params: {
										organization,
										project,
										namespace,
									},
									search: (old) => ({
										...(old as Record<string, unknown>),
										modal: "create-actor",
									}),
								});
							}}
						>
							<Icon icon={faPlusCircle} className="mr-2" />
							Create Actor
						</CommandItem>
					</CommandGroup>
				</CommandList>
			</Command>
		</div>
	);
}
