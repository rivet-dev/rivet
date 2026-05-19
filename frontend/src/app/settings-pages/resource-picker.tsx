import { faArrowRight, faFolder, faFolderTree, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { cn, SmallText } from "@/components";
import { useCloudDataProvider } from "@/components/actors/data-provider";

interface ResourcePickerProps {
	title: string;
	description: string;
	/** Modal search-param to preserve so the drawer stays open after nav. */
	modal: string;
	/** "project" → land on the project. "namespace" → require project + pick a namespace. */
	target: "project" | "namespace";
}

export function ResourcePicker({
	title,
	description,
	modal,
	target,
}: ResourcePickerProps) {
	const params = useParams({ strict: false }) as {
		organization?: string;
		project?: string;
	};
	const organization = params.organization;
	const project = params.project;

	if (!organization) {
		return (
			<div className="flex h-48 items-center justify-center rounded-lg border border-dashed border-border">
				<SmallText className="text-muted-foreground text-center">
					Open this from an organization to pick a {target}.
				</SmallText>
			</div>
		);
	}

	if (target === "namespace" && project) {
		return (
			<NamespacePicker
				organization={organization}
				project={project}
				title={title}
				description={description}
				modal={modal}
			/>
		);
	}

	// Two-step case: caller wants a namespace, but we don't have a project
	// in the URL yet. Show the project picker first with step-appropriate
	// copy (the caller's title/description describes the namespace step).
	const projectStepTitle =
		target === "namespace" ? "Pick a project" : title;
	const projectStepDescription =
		target === "namespace"
			? "Settings are scoped to a namespace. Choose a project first to find one."
			: description;

	return (
		<ProjectPicker
			organization={organization}
			title={projectStepTitle}
			description={projectStepDescription}
			modal={modal}
			target={target}
		/>
	);
}

function ProjectPicker({
	organization,
	title,
	description,
	modal,
	target,
}: {
	organization: string;
	title: string;
	description: string;
	modal: string;
	target: "project" | "namespace";
}) {
	const navigate = useNavigate();
	const dataProvider = useCloudDataProvider();
	const { data: projects = [], isLoading } = useInfiniteQuery(
		dataProvider.projectsQueryOptions({ organization }),
	);

	return (
		<div className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
			<header className="px-4 py-3 border-b border-foreground/10">
				<h3 className="text-sm font-semibold text-foreground">{title}</h3>
				<SmallText className="text-muted-foreground">
					{description}
				</SmallText>
			</header>
			{isLoading ? (
				<RowSkeletons />
			) : projects.length === 0 ? (
				<EmptyRow text="No projects in this organization." />
			) : (
				projects.map((p) => (
					<button
						key={p.id}
						type="button"
						onClick={() => {
							if (target === "project") {
								void navigate({
									to: "/orgs/$organization/projects/$project",
									params: {
										organization,
										project: p.name,
									},
									search: { modal },
								});
							} else {
								// For namespace target, land on the project — the
								// drawer's namespace tab will then show the
								// namespace picker for that project.
								void navigate({
									to: "/orgs/$organization/projects/$project",
									params: {
										organization,
										project: p.name,
									},
									search: { modal },
								});
							}
						}}
						className={cn(
							"w-full grid grid-cols-[auto_1fr_auto] items-center gap-3 px-4 py-3 text-left text-xs",
							"border-b border-foreground/10 last:border-b-0",
							"hover:bg-foreground/[0.04] focus-visible:outline-none focus-visible:bg-foreground/[0.06]",
						)}
					>
						<Icon
							icon={faFolder}
							className="size-3.5 text-muted-foreground shrink-0"
						/>
						<div className="min-w-0">
							<div className="text-sm font-medium text-foreground truncate">
								{p.displayName}
							</div>
							<div className="font-mono-console text-muted-foreground truncate text-[11px]">
								{p.name}
							</div>
						</div>
						<Icon
							icon={faArrowRight}
							className="size-3 text-muted-foreground/60 shrink-0"
						/>
					</button>
				))
			)}
		</div>
	);
}

function NamespacePicker({
	organization,
	project,
	title,
	description,
	modal,
}: {
	organization: string;
	project: string;
	title: string;
	description: string;
	modal: string;
}) {
	const navigate = useNavigate();
	const dataProvider = useCloudDataProvider();
	const { data: namespaces = [], isLoading } = useInfiniteQuery(
		dataProvider.orgProjectNamespacesQueryOptions({ organization, project }),
	);

	return (
		<div className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
			<header className="px-4 py-3 border-b border-foreground/10">
				<h3 className="text-sm font-semibold text-foreground">{title}</h3>
				<SmallText className="text-muted-foreground">
					{description}
				</SmallText>
			</header>
			{isLoading ? (
				<RowSkeletons />
			) : namespaces.length === 0 ? (
				<EmptyRow text="No namespaces in this project yet." />
			) : (
				namespaces.map((ns) => (
					<button
						key={ns.id}
						type="button"
						onClick={() => {
							void navigate({
								to: "/orgs/$organization/projects/$project/ns/$namespace",
								params: {
									organization,
									project,
									namespace: ns.name,
								},
								search: { modal },
							});
						}}
						className={cn(
							"w-full grid grid-cols-[auto_1fr_auto] items-center gap-3 px-4 py-3 text-left text-xs",
							"border-b border-foreground/10 last:border-b-0",
							"hover:bg-foreground/[0.04] focus-visible:outline-none focus-visible:bg-foreground/[0.06]",
						)}
					>
						<Icon
							icon={faFolderTree}
							className="size-3.5 text-muted-foreground shrink-0"
						/>
						<div className="min-w-0">
							<div className="text-sm font-medium text-foreground truncate">
								{ns.displayName}
							</div>
							<div className="font-mono-console text-muted-foreground truncate text-[11px]">
								{ns.name}
							</div>
						</div>
						<Icon
							icon={faArrowRight}
							className="size-3 text-muted-foreground/60 shrink-0"
						/>
					</button>
				))
			)}
		</div>
	);
}

function RowSkeletons() {
	return (
		<>
			{[0, 1, 2].map((i) => (
				<div
					// biome-ignore lint/suspicious/noArrayIndexKey: skeleton rows are static
					key={i}
					className="grid grid-cols-[auto_1fr] items-center gap-3 px-4 py-3 border-b border-foreground/10 last:border-b-0"
				>
					<div className="size-3.5 rounded-full bg-foreground/[0.06]" />
					<div className="h-3.5 w-32 rounded bg-foreground/[0.06]" />
				</div>
			))}
		</>
	);
}

function EmptyRow({ text }: { text: string }) {
	return (
		<div className="px-4 py-6 text-center">
			<SmallText className="text-muted-foreground">{text}</SmallText>
		</div>
	);
}
