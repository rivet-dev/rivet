import { useClerk } from "@clerk/clerk-react";
import { faChevronDown, faPlusCircle, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
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
import { useCloudDataProvider } from "@/components/actors";
import { SafeHover } from "@/components/safe-hover";
import { VisibilitySensor } from "@/components/visibility-sensor";

export function ContextSwitcher() {
	const [isOpen, setIsOpen] = useState(false);

	const match = useContextSwitchMatch();

	if (!match) {
		return null;
	}

	return (
		<>
			<Popover open={isOpen} onOpenChange={setIsOpen}>
				<PopoverTrigger asChild>
					<Button
						variant="outline"
						className="flex h-auto justify-between items-center px-2 py-1.5"
						endIcon={<Icon icon={faChevronDown} />}
					>
						<Breadcrumbs />
					</Button>
				</PopoverTrigger>
				<PopoverContent
					className="p-0 max-w-[calc(12rem*3)] w-full"
					align="start"
				>
					<Content onClose={() => setIsOpen(false)} />
				</PopoverContent>
			</Popover>
		</>
	);
}

const useContextSwitchMatch = ():
	| {
			project: string;
			namespace: string;
			organization: string;
	  }
	| { organization: string; project: string }
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

	return false;
};

function Breadcrumbs() {
	const match = useContextSwitchMatch();

	if (match && "project" in match && "namespace" in match) {
		return (
			<div className="flex flex-col items-center min-w-0 w-full">
				<div className="text-left text-xs text-muted-foreground min-w-0 flex w-full">
					<ProjectBreadcrumb
						project={match.project}
						className="truncate min-w-0 max-w-full block h-4"
					/>
				</div>
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
		return (
			<>
				<ProjectBreadcrumb project={match.project} />
			</>
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
	const clerk = useClerk();
	const project = useParams({
		strict: false,
		select(params) {
			return params.project;
		},
	});

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
										navigate({
											to: ".",
											search: (old) => ({
												...old,
												modal: "create-project",
											}),
										});
									}}
								>
									Create Project
								</Button>
							</CommandEmpty>
						) : null}

						{data?.map((project) => (
							<SafeHover key={project.id} offset={40}>
								<CommandItem
									value={project.name}
									keywords={[
										project.displayName,
										project.name,
									]}
									className="static w-full"
									onSelect={() => {
										clerk.setActive({
											organization,
										});
										navigate({
											to: "/orgs/$organization/projects/$project",
											params: {
												organization: organization,
												project: project.name,
											},
										});
										onClose?.();
									}}
									onMouseEnter={() => {
										onHover?.(project.name);
									}}
									onFocus={() => {
										onHover?.(project.name);
									}}
								>
									<span className="truncate w-full">
										{project.displayName}
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
							keywords={["create", "new", "project"]}
							onSelect={() => {
								onHover?.(null);
								navigate({
									to: ".",
									search: (old) => ({
										...old,
										modal: "create-project",
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
										navigate({
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

						{data?.map((namespace) => (
							<SafeHover key={namespace.id} offset={40}>
								<CommandItem
									value={namespace.name}
									keywords={[
										namespace.displayName,
										namespace.name,
									]}
									className="static w-full"
									onSelect={() => {
										clerk.setActive({
											organization,
										});
										navigate({
											to: "/orgs/$organization/projects/$project/ns/$namespace",
											params: {
												organization: organization,
												project: project,
												namespace: namespace.name,
											},
										});
										onClose?.();
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
								navigate({
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
