import { faGear, faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
	Button,
	cn,
	H1,
	RelativeTime,
	ScrollArea,
	SmallText,
	WithTooltip,
} from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";

export function NamespacesGrid({
	organization,
	project,
}: {
	organization: string;
	project: string;
}) {
	const navigate = useNavigate();
	const dataProvider = useCloudProjectDataProvider();
	const { data: namespaces = [], isLoading } = useInfiniteQuery(
		dataProvider.currentProjectNamespacesQueryOptions(),
	);
	const { data: projectData } = useQuery(
		dataProvider.currentProjectQueryOptions(),
	);

	const sorted = [...namespaces].sort((a, b) =>
		a.displayName.localeCompare(b.displayName),
	);

	const heading = projectData?.displayName
		? `${projectData.displayName} Namespaces`
		: "Namespaces";

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto">
					<div className="flex items-start justify-between gap-4 mb-6">
						<div>
							<H1 className="text-2xl">{heading}</H1>
							<SmallText className="text-muted-foreground mt-1">
								Each row is a namespace in the{" "}
								{projectData?.displayName ?? "this"} project — e.g.
								Local, Staging, Production.
							</SmallText>
						</div>
						<div className="flex items-center gap-2 shrink-0">
							{sorted.length > 0 ? (
								<Button
									variant="outline"
									size="sm"
									startIcon={<Icon icon={faPlus} />}
									onClick={() => {
										navigate({
											to: ".",
											search: (old) => ({
												...(old as Record<string, unknown>),
												modal: "create-ns",
											}),
										});
									}}
								>
									Create Namespace
								</Button>
							) : null}
							<WithTooltip
								content="Project settings"
								trigger={
									<Button
										variant="outline"
										size="icon-sm"
										aria-label="Project settings"
										onClick={() => {
											navigate({
												to: ".",
												search: (old) => ({
													...(old as Record<string, unknown>),
													modal: "billing",
												}),
											});
										}}
									>
										<Icon icon={faGear} />
									</Button>
								}
							/>
						</div>
					</div>

					{!isLoading && sorted.length === 0 ? (
						<div className="flex flex-col items-center gap-3 rounded-md border border-dashed bg-card/50 px-6 py-10 text-center">
							<H1 className="text-base">No namespaces yet</H1>
							<SmallText className="text-muted-foreground max-w-md">
								Create a namespace to start deploying actors in this
								project.
							</SmallText>
							<Button
								variant="default"
								size="sm"
								startIcon={<Icon icon={faPlus} />}
								onClick={() => {
									navigate({
										to: ".",
										search: (old) => ({
											...(old as Record<string, unknown>),
											modal: "create-ns",
										}),
									});
								}}
							>
								Create Namespace
							</Button>
						</div>
					) : (
						<div className="rounded-md border border-foreground/10 bg-card overflow-hidden">
							<div className="grid grid-cols-[1fr_1fr_160px] gap-4 px-3 py-2 text-[11px] font-medium uppercase tracking-wide text-muted-foreground border-b border-foreground/10 bg-foreground/[0.02]">
								<div>Name</div>
								<div>Identifier</div>
								<div>Created</div>
							</div>
							{sorted.map((ns) => (
								<Link
									key={ns.id}
									to="/orgs/$organization/projects/$project/ns/$namespace"
									params={{
										organization,
										project,
										namespace: ns.name,
									}}
									className={cn(
										"grid grid-cols-[1fr_1fr_160px] gap-4 items-center px-3 py-2.5 text-xs border-b border-foreground/10 last:border-b-0 transition-colors",
										"hover:bg-foreground/[0.04] focus-visible:outline-none focus-visible:bg-foreground/[0.06]",
									)}
								>
									<div className="font-medium text-foreground truncate">
										{ns.displayName}
									</div>
									<div className="font-mono-console text-muted-foreground truncate">
										{ns.name}
									</div>
									<div className="text-muted-foreground">
										{ns.createdAt ? (
											<RelativeTime
												time={new Date(ns.createdAt)}
											/>
										) : (
											"—"
										)}
									</div>
								</Link>
							))}
						</div>
					)}
				</div>
			</ScrollArea>
		</div>
	);
}
