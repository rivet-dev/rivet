import { useClerk, useOrganizationList } from "@clerk/clerk-react";
import { AvatarImage } from "@radix-ui/react-avatar";
import { faPlusCircle, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useMatchRoute, useNavigate, useParams } from "@tanstack/react-router";
import { useState } from "react";
import {
	Avatar,
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
import { useManager } from "@/components/actors";
import { SafeHover } from "@/components/safe-hover";
import { VisibilitySensor } from "@/components/visibility-sensor";
import {
	namespaceQueryOptions,
	organizationQueryOptions,
	projectQueryOptions,
	projectsQueryOptions,
} from "@/queries/manager-cloud";

export function ContextSwitcher() {
	const [isOpen, setIsOpen] = useState(false);

	return (
		<Popover open={isOpen} onOpenChange={setIsOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="outline"
					className="flex flex-col h-auto justify-start items-start"
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
	);
}

function Breadcrumbs() {
	const match = useMatchRoute();

	const matchNamespace = match({
		to: "/orgs/$organization/projects/$project/ns/$namespace",
		fuzzy: true,
	});
	if (matchNamespace) {
		return (
			<>
				<OrganizationBreadcrumb
					org={matchNamespace.organization}
					className="text-xs [&>span]:size-4 mb-1"
				/>
				<ProjectBreadcrumb
					project={matchNamespace.project}
					className="text-xs [&>span]:size-4 mb-1"
				/>
				<NamespaceBreadcrumb
					namespace={matchNamespace.namespace}
					project={matchNamespace.project}
				/>
			</>
		);
	}

	const matchProject = match({
		to: "/orgs/$organization/projects/$project",
	});

	if (matchProject) {
		return (
			<>
				<OrganizationBreadcrumb
					org={matchProject.organization}
					className="text-xs [&>span]:size-4 mb-1"
				/>
				<ProjectBreadcrumb project={matchProject.project} />
			</>
		);
	}

	const matchOrg = match({
		to: "/orgs/$organization",
	});

	if (matchOrg) {
		return <OrganizationBreadcrumb org={matchOrg.organization} />;
	}
}

function OrganizationBreadcrumb({
	org,
	className,
}: {
	org: string;
	className?: string;
}) {
	const { isLoading, data } = useQuery(organizationQueryOptions({ org }));
	if (isLoading) {
		return <Skeleton className="h-5 w-32" />;
	}

	return (
		<div className={cn("flex justify-start", className)}>
			<Avatar className="size-5 mr-1">
				<AvatarImage
					src={data?.imageUrl}
					alt={data?.name || "Organization Avatar"}
				/>
			</Avatar>
			<span>{data?.name}</span>
		</div>
	);
}

function ProjectBreadcrumb({
	project,
	className,
}: {
	project: string;
	className?: string;
}) {
	const { isLoading, data } = useQuery(projectQueryOptions({ project }));
	if (isLoading) {
		return <Skeleton className="h-5 w-32" />;
	}

	return <span className={className}>{data?.name}</span>;
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
		namespaceQueryOptions({ project, namespace }),
	);
	if (isLoading) {
		return <Skeleton className="h-5 w-32" />;
	}

	return <span className={className}>{data?.name}</span>;
}

function Content({ onClose }: { onClose?: () => void }) {
	const params = useParams({ strict: false });
	const {
		userMemberships: {
			data: userMemberships = [],
			isLoading,
			hasNextPage,
			fetchNext,
		},
	} = useOrganizationList({
		userMemberships: {
			infinite: true,
		},
	});
	const clerk = useClerk();

	const [currentOrgHover, setCurrentOrgHover] = useState<string | null>(
		params.organization || null,
	);

	const [currentProjectHover, setCurrentProjectHover] = useState<
		string | null
	>(params.project || null);

	const navigate = useNavigate();

	return (
		<div className="flex w-full">
			<div className="w-48">
				<Command loop defaultValue={clerk.organization?.id}>
					<CommandInput
						className="text-sm"
						placeholder="Find Organization..."
					/>
					<CommandList className="relative p-1">
						<CommandGroup heading="Organizations">
							{!isLoading ? (
								<CommandEmpty>
									No organizations found.
									<Button
										className="mt-1"
										variant="outline"
										size="sm"
										startIcon={<Icon icon={faPlusCircle} />}
										onClick={() => {
											onClose?.();
											clerk.openCreateOrganization();
										}}
									>
										Create Organization
									</Button>
								</CommandEmpty>
							) : null}
							{userMemberships.map((membership) => (
								<SafeHover key={membership.id} offset={40}>
									<CommandItem
										onSelect={() => {
											clerk.setActive({
												organization:
													membership.organization.id,
											});
											navigate({
												to: "/orgs/$organization",
												params: {
													organization:
														membership.organization
															.id,
												},
											});
											onClose?.();
										}}
										value={membership.organization.id}
										onMouseEnter={() => {
											setCurrentOrgHover(
												membership.organization.id,
											);
											setCurrentProjectHover(null);
										}}
										keywords={[
											membership.organization.name,
										]}
										className="static cursor-pointer"
									>
										{membership.organization.name}
									</CommandItem>
								</SafeHover>
							))}
							{isLoading ? (
								<>
									<ListItemSkeleton />
									<ListItemSkeleton />
									<ListItemSkeleton />
									<ListItemSkeleton />
									<ListItemSkeleton />
								</>
							) : null}
							<CommandItem
								keywords={[
									"create",
									"new",
									"organization",
									"team",
								]}
								onMouseEnter={() => {
									setCurrentOrgHover(null);
									setCurrentProjectHover(null);
								}}
								onFocus={() => {
									setCurrentOrgHover(null);
									setCurrentProjectHover(null);
								}}
								onSelect={() => {
									clerk.openCreateOrganization();
								}}
							>
								<Icon icon={faPlusCircle} className="mr-2" />
								Create Organization
							</CommandItem>

							{hasNextPage ? (
								<VisibilitySensor onChange={fetchNext} />
							) : null}
						</CommandGroup>
					</CommandList>
				</Command>
			</div>
			{currentOrgHover ? (
				<ProjectList
					organization={currentOrgHover}
					onHover={setCurrentProjectHover}
					onClose={onClose}
				/>
			) : null}
			{currentProjectHover && currentOrgHover ? (
				<NamespaceList
					organization={currentOrgHover}
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
		useInfiniteQuery(projectsQueryOptions({ organization: organization }));
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
		useInfiniteQuery(useManager().projectNamespacesQueryOptions(project));
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
