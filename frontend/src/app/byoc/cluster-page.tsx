import type { Rivet } from "@rivet-gg/cloud";
import {
	faArrowUpRightFromSquare,
	faCalendarDays,
	faChevronDown,
	faChevronRight,
	faCopy,
	faDownload,
	faEnvelope,
	faKey,
	faPlus,
	faSlack,
	faTrash,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { saveAs } from "file-saver";
import type { ReactNode } from "react";
import { useState } from "react";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import {
	Badge,
	Button,
	CopyTrigger,
	cn,
	H1,
	MultiSelectFormField,
	RelativeTime,
	ScrollArea,
	Skeleton,
	SmallText,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	toast,
	WithTooltip,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import {
	BYOC_QUICKSTART_DOCS_URL,
	BYOC_SETUP_KIT_URL,
	BYOC_SUPPORT_EMAIL,
} from "@/content/byoc";
import { cloudEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";
import { serializeAgentInstructions } from "./agent-instructions";
import { ByocContactTrigger } from "./byoc-contact-trigger";
import {
	CLUSTER_CONFIG_FILENAME,
	serializeClusterConfig,
} from "./cluster-config";

type Command = Rivet.ByocListCommandsResponse.Commands.Item;
type Region = Rivet.ByocListRegionsResponse.Regions.Item;

const CLUSTER_ROUTE_ID = "/_context/orgs/$organization/clusters/$cluster";

export function ClusterPage({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const { data } = useQuery(
		dataProvider.currentOrgClusterQueryOptions({ cluster }),
	);
	const regions = useInfiniteQuery(
		dataProvider.currentOrgClusterRegionsQueryOptions({ cluster }),
	);
	const regionCount = regions.data?.length;

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto space-y-6">
					<header className="flex flex-wrap items-start justify-between gap-4">
						<div className="min-w-0">
							<div className="flex items-center gap-2">
								<H1 className="text-2xl truncate">
									{data?.name ?? cluster}
								</H1>
								<PlanBadge plan="byoc" />
							</div>
							<SmallText className="mt-1 text-muted-foreground">
								<ClusterStatus
									regionCount={regionCount}
									isLoading={regions.isLoading}
								/>
							</SmallText>
						</div>
						<div className="flex shrink-0 items-center gap-2">
							<ByocContactTrigger>
								{(open) => (
									<Button
										variant="outline"
										size="sm"
										startIcon={
											<Icon icon={faCalendarDays} />
										}
										onClick={open}
									>
										Book a call
									</Button>
								)}
							</ByocContactTrigger>
							<Button
								asChild
								variant="outline"
								size="sm"
								endIcon={
									<Icon icon={faArrowUpRightFromSquare} />
								}
							>
								<a
									href={BYOC_QUICKSTART_DOCS_URL}
									target="_blank"
									rel="noreferrer"
								>
									Setup guide
								</a>
							</Button>
						</div>
					</header>

					<div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_17rem]">
						<div className="min-w-0 space-y-6">
							<SetupSection
								cluster={cluster}
								clusterId={data?.id}
								hasRegions={!!regionCount}
							/>
							<ActivitySection
								cluster={cluster}
								regionCount={regionCount}
							/>
							<OtelTokenSection key={cluster} cluster={cluster} />
						</div>
						<aside className="space-y-6">
							<ResourcesCard
								cluster={cluster}
								clusterId={data?.id}
							/>
							<SupportCard />
						</aside>
					</div>
				</div>
			</ScrollArea>
		</div>
	);
}

ClusterPage.Skeleton = function ClusterPageSkeleton() {
	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<div className="px-6 py-6 max-w-6xl mx-auto w-full space-y-6">
				<Skeleton className="h-8 w-64" />
				<div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_17rem]">
					<div className="space-y-6">
						<Skeleton className="h-48 w-full rounded-lg" />
						<Skeleton className="h-64 w-full rounded-lg" />
					</div>
					<div className="space-y-6">
						<Skeleton className="h-40 w-full rounded-lg" />
						<Skeleton className="h-32 w-full rounded-lg" />
					</div>
				</div>
			</div>
		</div>
	);
};

function ClusterStatus({
	regionCount,
	isLoading,
}: {
	regionCount: number | undefined;
	isLoading: boolean;
}) {
	if (isLoading || regionCount === undefined) {
		return <Skeleton className="inline-block h-3.5 w-48 align-middle" />;
	}
	if (regionCount === 0) {
		return (
			<>
				<StatusDot className="bg-muted-foreground/60" />
				No regions
			</>
		);
	}
	return (
		<>
			<StatusDot className="bg-emerald-500" />
			{regionCount} {regionCount === 1 ? "region" : "regions"}
		</>
	);
}

function StatusDot({ className }: { className: string }) {
	return (
		<span
			aria-hidden="true"
			className={cn(
				"mr-1.5 inline-block size-1.5 rounded-full align-middle",
				className,
			)}
		/>
	);
}

function Card({
	title,
	description,
	action,
	className,
	children,
}: {
	title?: string;
	description?: string;
	action?: ReactNode;
	className?: string;
	children: ReactNode;
}) {
	return (
		<section
			className={cn(
				"rounded-lg border border-foreground/10 bg-foreground/[0.02]",
				className,
			)}
		>
			{title ? (
				<div className="flex items-start justify-between gap-4 px-5 pt-5">
					<div className="min-w-0">
						<h2 className="text-base font-semibold text-foreground">
							{title}
						</h2>
						{description ? (
							<SmallText className="mt-1 text-muted-foreground">
								{description}
							</SmallText>
						) : null}
					</div>
					{action}
				</div>
			) : null}
			{children}
		</section>
	);
}

function SetupSection({
	cluster,
	clusterId,
	hasRegions,
}: {
	cluster: string;
	clusterId: string | undefined;
	hasRegions: boolean;
}) {
	const dataProvider = useCloudDataProvider();
	const { data, isLoading } = useQuery(
		dataProvider.currentOrgClusterOperatorTokenQueryOptions({ cluster }),
	);
	const agentInstructions = serializeAgentInstructions(
		clusterId,
		cloudEnv().VITE_APP_CLOUD_API_URL,
		data?.token,
	);

	return (
		<Card
			title={hasRegions ? "Add a region" : "Setup"}
			description="Run Rivet in your VPC, fully managed by Rivet."
		>
			<div className="grid gap-3 p-5 sm:grid-cols-2">
				<SetupOption
					recommended
					title="Use your coding agent"
					description="Paste instructions into your coding agent."
					footnote="Includes your operator token. Keep it secret."
				>
					<CopyTrigger value={agentInstructions ?? ""}>
						<Button
							className="w-full"
							startIcon={<Icon icon={faCopy} />}
							disabled={!agentInstructions}
							isLoading={isLoading}
						>
							Copy agent instructions
						</Button>
					</CopyTrigger>
				</SetupOption>
				<SetupOption
					title="Follow the setup guide"
					description="Download the kit and cluster config to provision Rivet in your cloud."
				>
					<Button
						asChild
						variant="outline"
						className="w-full"
						endIcon={<Icon icon={faArrowUpRightFromSquare} />}
					>
						<a
							href={BYOC_QUICKSTART_DOCS_URL}
							target="_blank"
							rel="noreferrer"
						>
							Setup guide
						</a>
					</Button>
				</SetupOption>
			</div>
		</Card>
	);
}

function SetupOption({
	recommended,
	title,
	description,
	footnote,
	children,
}: {
	recommended?: boolean;
	title: string;
	description: string;
	footnote?: string;
	children: ReactNode;
}) {
	return (
		<div
			className={cn(
				"relative flex flex-col rounded-lg border bg-card p-4",
				recommended ? "border-primary" : "border-border",
			)}
		>
			{recommended ? (
				<Badge className="absolute -top-2.5 left-4 bg-card">
					Recommended
				</Badge>
			) : null}
			<p className="font-medium">{title}</p>
			<p className="mt-1 text-sm text-muted-foreground">{description}</p>
			<div className="mt-4 flex-1" />
			{children}
			{footnote ? (
				<p className="mt-2 text-xs text-muted-foreground">{footnote}</p>
			) : null}
		</div>
	);
}

function ResourcesCard({
	cluster,
	clusterId,
}: {
	cluster: string;
	clusterId: string | undefined;
}) {
	const dataProvider = useCloudDataProvider();
	const { data, isLoading, isError } = useQuery(
		dataProvider.currentOrgClusterOperatorTokenQueryOptions({ cluster }),
	);
	const config = serializeClusterConfig(
		clusterId,
		cloudEnv().VITE_APP_CLOUD_API_URL,
	);

	return (
		<Card title="Resources">
			<ul className="mt-3 divide-y divide-foreground/10 border-t border-foreground/10 text-sm">
				<ResourceRow
					icon={faDownload}
					href={BYOC_SETUP_KIT_URL}
					download
				>
					Download kit
				</ResourceRow>
				<ResourceRow
					icon={faDownload}
					disabled={!config}
					onClick={() => {
						if (config)
							saveAs(
								new Blob([config], {
									type: "application/json;charset=utf-8",
								}),
								CLUSTER_CONFIG_FILENAME,
							);
					}}
				>
					Download config
				</ResourceRow>
				<WithTooltip
					disabled={isLoading || !!data?.token}
					content={
						isError
							? "Could not load the operator token. Reload to retry or contact Enterprise Support."
							: "No active operator token. Contact Enterprise Support."
					}
					trigger={
						<li className="flex">
							<CopyTrigger value={data?.token ?? ""}>
								<ResourceButton
									icon={faKey}
									disabled={isLoading || !data?.token}
								>
									Copy token
								</ResourceButton>
							</CopyTrigger>
						</li>
					}
				/>
			</ul>
		</Card>
	);
}

const RESOURCE_ROW_CLASS =
	"flex w-full items-center gap-3 px-5 py-2.5 text-left text-foreground transition-colors hover:bg-foreground/[0.04] disabled:cursor-not-allowed disabled:opacity-50";

function ResourceRow({
	icon,
	href,
	download,
	disabled,
	onClick,
	children,
}: {
	icon: typeof faDownload;
	href?: string;
	download?: boolean;
	disabled?: boolean;
	onClick?: () => void;
	children: ReactNode;
}) {
	return (
		<li className="flex">
			{href ? (
				<a
					href={href}
					download={download}
					className={RESOURCE_ROW_CLASS}
				>
					<Icon icon={icon} className="text-muted-foreground" />
					{children}
				</a>
			) : (
				<ResourceButton
					icon={icon}
					disabled={disabled}
					onClick={onClick}
				>
					{children}
				</ResourceButton>
			)}
		</li>
	);
}

function ResourceButton({
	icon,
	disabled,
	onClick,
	children,
}: {
	icon: typeof faDownload;
	disabled?: boolean;
	onClick?: () => void;
	children: ReactNode;
}) {
	return (
		<button
			type="button"
			className={RESOURCE_ROW_CLASS}
			disabled={disabled}
			onClick={onClick}
		>
			<Icon icon={icon} className="text-muted-foreground" />
			{children}
		</button>
	);
}

function SupportCard() {
	return (
		<Card
			title="Enterprise Support"
			description="Every BYOC cluster includes enterprise support."
		>
			<ul className="mt-3 divide-y divide-foreground/10 border-t border-foreground/10 text-sm">
				<li className="flex">
					<ByocContactTrigger>
						{(open) => (
							<ResourceButton icon={faSlack} onClick={open}>
								Slack Connect
							</ResourceButton>
						)}
					</ByocContactTrigger>
				</li>
				<ResourceRow
					icon={faEnvelope}
					href={`mailto:${BYOC_SUPPORT_EMAIL}`}
				>
					{BYOC_SUPPORT_EMAIL}
				</ResourceRow>
			</ul>
		</Card>
	);
}

function ActivitySection({
	cluster,
	regionCount,
}: {
	cluster: string;
	regionCount: number | undefined;
}) {
	return (
		<Tabs defaultValue="regions" asChild>
			<Card>
				<TabsList className="px-3">
					<TabsTrigger value="regions">
						Regions
						{regionCount ? (
							<span className="ml-1.5 rounded-sm bg-foreground/[0.06] px-1.5 py-0.5 font-mono-console text-[11px] font-normal text-muted-foreground">
								{regionCount}
							</span>
						) : null}
					</TabsTrigger>
					<TabsTrigger value="commands">Commands</TabsTrigger>
				</TabsList>
				<TabsContent value="regions" className="mt-0 p-4">
					<RegionsPanel cluster={cluster} />
				</TabsContent>
				<TabsContent value="commands" className="mt-0 p-4">
					<CommandsPanel cluster={cluster} />
				</TabsContent>
			</Card>
		</Tabs>
	);
}

const REGION_COLUMNS = "grid-cols-[minmax(0,1fr)_9rem_9rem]";
const COMMAND_COLUMNS = "grid-cols-[1.5rem_minmax(0,1fr)_minmax(0,12rem)_6rem]";

function TableHead({
	columns,
	children,
}: {
	columns: string;
	children: React.ReactNode;
}) {
	return (
		<div
			className={cn(
				"grid gap-4 px-3 py-2 text-[11px] font-medium uppercase tracking-wide text-muted-foreground border-b border-foreground/10 bg-foreground/[0.02]",
				columns,
			)}
		>
			{children}
		</div>
	);
}

function TableMessage({
	children,
	onRetry,
}: {
	children: React.ReactNode;
	onRetry?: () => void;
}) {
	return (
		<div className="px-3 py-4 flex flex-col items-center gap-2">
			<SmallText className="text-muted-foreground">{children}</SmallText>
			{onRetry ? (
				<Button variant="outline" size="sm" onClick={onRetry}>
					Retry
				</Button>
			) : null}
		</div>
	);
}

function ShowMore({
	isLoading,
	onClick,
}: {
	isLoading: boolean;
	onClick: () => void;
}) {
	return (
		<div className="mt-3 flex justify-center">
			<Button
				variant="outline"
				size="sm"
				isLoading={isLoading}
				onClick={onClick}
			>
				Show more
			</Button>
		</div>
	);
}

function RegionsPanel({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const {
		data,
		isLoading,
		isError,
		refetch,
		hasNextPage,
		fetchNextPage,
		isFetchingNextPage,
	} = useInfiniteQuery(
		dataProvider.currentOrgClusterRegionsQueryOptions({ cluster }),
	);

	return (
		<>
			<div className="rounded-md border border-foreground/10 overflow-hidden">
				<TableHead columns={REGION_COLUMNS}>
					<div>Region</div>
					<div>Restarted</div>
					<div>Last seen</div>
				</TableHead>
				{isLoading ? (
					<div className="px-3 py-2.5">
						<Skeleton className="h-4 w-full" />
					</div>
				) : null}
				{isError ? (
					<TableMessage onRetry={() => void refetch()}>
						Couldn't load regions.
					</TableMessage>
				) : null}
				{data?.map((region) => (
					<RegionRow key={region.id} region={region} />
				))}
				{!isLoading && !isError && data?.length === 0 ? (
					<TableMessage>
						No regions have reported in yet.
					</TableMessage>
				) : null}
			</div>
			{hasNextPage ? (
				<ShowMore
					isLoading={isFetchingNextPage}
					onClick={() => void fetchNextPage()}
				/>
			) : null}
		</>
	);
}

function RegionRow({ region }: { region: Region }) {
	return (
		<div
			className={cn(
				"grid gap-4 items-center px-3 py-2.5 text-xs border-b border-foreground/10 last:border-b-0",
				REGION_COLUMNS,
			)}
		>
			<div className="font-medium text-foreground truncate">
				{region.name}
			</div>
			<div className="text-muted-foreground">
				<Time value={region.operatorBootId ?? undefined} />
			</div>
			<div className="text-muted-foreground">
				<Time value={region.lastSeenAt} />
			</div>
		</div>
	);
}

const STATUS_LABELS: Record<string, string> = {
	completed: "Succeeded",
	failed: "Failed",
	cancelled: "Cancelled",
};

function commandStatus(command: Command) {
	if (command.status) {
		return STATUS_LABELS[command.status] ?? command.status;
	}
	return command.seenAt ? "Running" : "Pending";
}

function statusClassName(command: Command) {
	if (command.status === "failed") return "text-destructive";
	if (command.status === "completed") return "text-foreground";
	return "text-muted-foreground";
}

function RegionFilter({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const navigate = useNavigate();
	const regions = useRegionFilter();
	const { data } = useInfiniteQuery(
		dataProvider.currentOrgClusterRegionsQueryOptions({ cluster }),
	);

	return (
		<div className="w-56 shrink-0">
			<MultiSelectFormField
				placeholder="All regions"
				defaultValue={regions}
				options={(data ?? []).map((region) => ({
					label: region.name,
					value: region.name,
				}))}
				onValueChange={(values) => {
					void navigate({
						to: ".",
						search: (old) => ({
							...old,
							regions: values.length ? values : undefined,
						}),
					});
				}}
			/>
		</div>
	);
}

function useRegionFilter() {
	return (
		useSearch({
			from: CLUSTER_ROUTE_ID,
			select: (search) => search.regions,
		}) ?? []
	);
}

function CommandsPanel({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const regions = useRegionFilter();
	const {
		data,
		isLoading,
		isError,
		refetch,
		hasNextPage,
		fetchNextPage,
		isFetchingNextPage,
	} = useInfiniteQuery(
		dataProvider.currentOrgClusterCommandsQueryOptions({
			cluster,
			regions,
		}),
	);

	return (
		<>
			<div className="mb-3 flex flex-wrap items-center justify-between gap-3">
				<SmallText className="text-muted-foreground">
					Issued by Rivet Cloud to operate and upgrade your cluster.
				</SmallText>
				<RegionFilter cluster={cluster} />
			</div>
			<div className="rounded-md border border-foreground/10 overflow-hidden">
				<TableHead columns={COMMAND_COLUMNS}>
					<div />
					<div>Kind</div>
					<div>Region</div>
					<div className="text-right">Status</div>
				</TableHead>
				{isLoading ? (
					<div className="px-3 py-2.5">
						<Skeleton className="h-4 w-full" />
					</div>
				) : null}
				{isError ? (
					<TableMessage onRetry={() => void refetch()}>
						Couldn't load commands.
					</TableMessage>
				) : null}
				{data?.map((command) => (
					<CommandRow key={command.id} command={command} />
				))}
				{!isLoading && !isError && data?.length === 0 ? (
					<TableMessage>
						{regions.length
							? "No commands for the selected regions."
							: "No commands have been issued yet."}
					</TableMessage>
				) : null}
			</div>
			{hasNextPage ? (
				<ShowMore
					isLoading={isFetchingNextPage}
					onClick={() => void fetchNextPage()}
				/>
			) : null}
		</>
	);
}

function CommandRow({ command }: { command: Command }) {
	const [isExpanded, setIsExpanded] = useState(false);

	return (
		<div className="border-b border-foreground/10 last:border-b-0">
			<button
				type="button"
				onClick={() => setIsExpanded((value) => !value)}
				className={cn(
					"grid w-full gap-4 items-center px-3 py-2.5 text-xs text-left transition-colors hover:bg-foreground/[0.025]",
					COMMAND_COLUMNS,
				)}
			>
				<Icon
					icon={isExpanded ? faChevronDown : faChevronRight}
					className="text-muted-foreground"
				/>
				<span className="capitalize text-foreground truncate">
					{command.type}
				</span>
				<span className="text-muted-foreground truncate">
					{command.region}
				</span>
				<span className={cn("text-right", statusClassName(command))}>
					{commandStatus(command)}
				</span>
			</button>
			{isExpanded ? (
				<div className="grid grid-cols-[8rem_minmax(0,1fr)] items-start gap-x-4 gap-y-2 pr-3 pt-3 pb-4 pl-[3.25rem] text-xs">
					<Timestamp label="Created" value={command.insertedAt} />
					<Timestamp label="Pulled" value={command.seenAt} />
					<Timestamp
						label={
							command.status === "failed" ? "Failed" : "Finished"
						}
						value={command.completedAt}
					/>
					<Payload label="Request payload" value={command.body} />
					<Payload label="Response payload" value={undefined} />
				</div>
			) : null}
		</div>
	);
}

function Timestamp({
	label,
	value,
}: {
	label: string;
	value: string | undefined;
}) {
	return (
		<>
			<span className="text-muted-foreground">{label}</span>
			<span className="text-foreground">
				<Time value={value} />
			</span>
		</>
	);
}

function Payload({
	label,
	value,
}: {
	label: string;
	value: string | undefined;
}) {
	return (
		<>
			<span className={cn("text-muted-foreground", value && "pt-2")}>
				{label}
			</span>
			{value ? (
				<pre className="overflow-x-auto rounded-md border border-foreground/10 bg-background p-2 font-mono-console">
					{value}
				</pre>
			) : (
				<span className="text-muted-foreground">—</span>
			)}
		</>
	);
}

function Time({ value }: { value: string | number | undefined }) {
	if (value === undefined || value === null) {
		return <>—</>;
	}
	const date = new Date(value);
	return (
		<WithTooltip
			content={date.toLocaleString(undefined, {
				dateStyle: "full",
				timeStyle: "long",
			})}
			trigger={
				<span>
					<RelativeTime time={date} />
				</span>
			}
		/>
	);
}

function OtelTokenSection({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const organization = dataProvider.organization;
	const { data, isLoading, isError } = useQuery(
		dataProvider.currentOrgClusterOtelTokenQueryOptions({ cluster }),
	);

	const [confirmingRevoke, setConfirmingRevoke] = useState(false);

	const invalidateToken = () =>
		queryClient.invalidateQueries({
			queryKey: [{ organization, cluster }, "byoc-otel-token"],
		});

	const createMutation = useMutation({
		...dataProvider.createOtelTokenMutationOptions(),
		onSuccess: () => {
			setConfirmingRevoke(false);
			void invalidateToken();
		},
		onError: () => {
			toast.error(
				"Could not create the metrics token. Please try again.",
			);
		},
	});

	const revokeMutation = useMutation({
		...dataProvider.revokeOtelTokenMutationOptions(),
		onSuccess: () => {
			setConfirmingRevoke(false);
			toast.success("Metrics ingest token revoked.");
			void invalidateToken();
		},
		onError: () => {
			toast.error(
				"Could not revoke the metrics token. Please try again.",
			);
		},
	});

	// The plaintext token is returned only once, at creation. Show it until the
	// user dismisses it, then fall back to the redacted view. This (and the
	// revoke-confirm state) is per-cluster; the parent remounts this component
	// via key={cluster} so navigating to another cluster cannot surface a
	// previous cluster's token or a stale revoke confirmation.
	const freshToken = createMutation.data?.token;

	// Shared so the token block is the same full-width box in every state and
	// does not resize when switching between the plaintext and redacted views.
	const tokenBoxClassName =
		"w-full overflow-x-auto rounded-md border border-foreground/10 bg-background p-2.5 font-mono-console text-sm";

	return (
		<Card
			title="Metrics ingest token"
			description="Authorizes sending OpenTelemetry metrics from this cluster to Rivet. Keep it secret."
		>
			<div className="px-5 pb-5 pt-4">
				{freshToken ? (
					<div className="flex flex-col gap-3">
						<div className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/[0.06] px-3 py-2 text-sm text-amber-600 dark:text-amber-400">
							<Icon
								icon={faTriangleExclamation}
								className="mt-0.5 shrink-0"
							/>
							<span>
								Copy this token now. For security it is only
								shown once and cannot be retrieved later.
							</span>
						</div>
						<pre className={tokenBoxClassName}>{freshToken}</pre>
						<div className="flex items-center justify-end gap-2">
							<Button
								variant="ghost"
								onClick={() => createMutation.reset()}
							>
								Done
							</Button>
							<CopyTrigger value={freshToken}>
								<Button
									variant="outline"
									startIcon={<Icon icon={faCopy} />}
								>
									Copy token
								</Button>
							</CopyTrigger>
						</div>
					</div>
				) : isLoading ? (
					<Skeleton className="h-24 w-full rounded-md" />
				) : isError ? (
					<SmallText className="text-muted-foreground">
						Could not load the metrics token. Reload to retry.
					</SmallText>
				) : data ? (
					<div className="flex flex-col gap-3">
						<pre className={tokenBoxClassName}>
							{`byoc_otel_${"•".repeat(16)}${data.tokenLastFour}`}
						</pre>
						<div className="flex flex-col items-start justify-between gap-2 sm:flex-row sm:items-center">
							<SmallText className="text-muted-foreground">
								Created <Time value={data.createdAt} />
								{data.lastUsedAt ? (
									<>
										{" · last used "}
										<Time value={data.lastUsedAt} />
									</>
								) : (
									" · never used"
								)}
							</SmallText>
							<div className="flex shrink-0 items-center gap-2">
								{confirmingRevoke ? (
									<>
										<Button
											key="cancel"
											variant="ghost"
											disabled={revokeMutation.isPending}
											onClick={() =>
												setConfirmingRevoke(false)
											}
										>
											Cancel
										</Button>
										<Button
											key="confirm"
											variant="destructive"
											startIcon={<Icon icon={faTrash} />}
											isLoading={revokeMutation.isPending}
											onClick={() =>
												revokeMutation.mutate({
													organization,
													cluster,
												})
											}
										>
											Confirm revoke
										</Button>
									</>
								) : (
									<Button
										key="revoke"
										variant="destructive-outline"
										startIcon={<Icon icon={faTrash} />}
										onClick={() =>
											setConfirmingRevoke(true)
										}
									>
										Revoke
									</Button>
								)}
							</div>
						</div>
					</div>
				) : (
					<div className="flex justify-center">
						<Button
							variant="outline"
							startIcon={<Icon icon={faPlus} />}
							isLoading={createMutation.isPending}
							onClick={() =>
								createMutation.mutate({ organization, cluster })
							}
						>
							Create token
						</Button>
					</div>
				)}
			</div>
		</Card>
	);
}
