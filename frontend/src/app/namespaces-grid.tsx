import { faGear, faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
	Button,
	cn,
	DiscreteCopyButton,
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

	const projectName = projectData?.displayName ?? "Project";

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto space-y-8">
					<header className="flex items-center justify-between gap-4 pb-6 border-b border-foreground/10">
						<H1 className="text-2xl truncate">{projectName}</H1>
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
												settings: "billing",
											}),
										});
									}}
								>
									<Icon icon={faGear} />
								</Button>
							}
						/>
					</header>

					<section>
						<header className="flex items-center justify-between gap-4 mb-3">
							<h2 className="text-base font-semibold text-foreground">
								Namespaces
							</h2>
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
						</header>

						{!isLoading && sorted.length === 0 ? (
							<div className="flex flex-col items-center gap-3 rounded-md border border-dashed bg-card/50 px-6 py-10 text-center">
								<h3 className="text-base font-semibold text-foreground">
									No namespaces yet
								</h3>
								<SmallText className="text-muted-foreground max-w-md">
									Create a namespace to start deploying actors in
									this project.
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
										<DiscreteCopyButton
											size="xs"
											value={ns.name}
											className="-mx-2 h-auto w-fit font-mono-console text-muted-foreground"
											onClick={(e) => e.preventDefault()}
										>
											<span className="truncate">{ns.name}</span>
										</DiscreteCopyButton>
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
					</section>
				</div>
			</ScrollArea>
		</div>
	);
}
