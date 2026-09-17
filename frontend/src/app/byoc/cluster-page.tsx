import {
	CLUSTER_CONFIG_FILENAME,
	serializeClusterConfig,
} from "./cluster-config";
import { cloudEnv } from "@/lib/env";
import type { Rivet } from "@rivet-gg/cloud";
import {
	faArrowUpRightFromSquare,
	faArrowRight,
	faCalendarDays,
	faChevronDown,
	faChevronRight,
	faCopy,
	faEnvelope,
	faPlus,
	faSlack,
	faTrash,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { saveAs } from "file-saver";
import { useState } from "react";
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
	toast,
	WithTooltip,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";
import {
	BYOC_QUICKSTART_DOCS_URL,
	BYOC_SETUP_KIT_URL,
	BYOC_SUPPORT_EMAIL,
} from "@/content/byoc";
import { ByocContactTrigger } from "./byoc-contact-trigger";
import { serializeAgentInstructions } from "./agent-instructions";
import { AgentPromptBanner } from "@/app/compute-deploy";
import { OrDivider } from "@/app/getting-started";

type Command = Rivet.ByocListCommandsResponse.Commands.Item;
type Region = Rivet.ByocListRegionsResponse.Regions.Item;

const CLUSTER_ROUTE_ID = "/_context/orgs/$organization/clusters/$cluster";

export function ClusterPage({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const { data } = useQuery(
		dataProvider.currentOrgClusterQueryOptions({ cluster }),
	);

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-4xl mx-auto space-y-6">
					<header className="flex items-start justify-between gap-4 pb-6 border-b border-foreground/10">
						<div className="min-w-0">
							<div className="flex items-center gap-2">
								<H1 className="text-2xl truncate">
									{data?.name ?? cluster}
								</H1>
								<Badge variant="premium-violet">BYOC</Badge>
							</div>
						</div>
					</header>

					<SetupSection cluster={cluster} clusterId={data?.id} />
					<RegionsSection cluster={cluster} />
					<CommandsSection cluster={cluster} />
					<OtelTokenSection key={cluster} cluster={cluster} />
					<SupportSection />
				</div>
			</ScrollArea>
		</div>
	);
}

ClusterPage.Skeleton = function ClusterPageSkeleton() {
	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<div className="px-6 py-6 max-w-4xl mx-auto w-full space-y-6">
				<Skeleton className="h-8 w-64" />
				{Array.from({ length: 3 }).map((_, index) => (
					<Skeleton
						// biome-ignore lint/suspicious/noArrayIndexKey: static skeleton sections
						key={index}
						className="h-40 w-full rounded-lg"
					/>
				))}
			</div>
		</div>
	);
};

function Section({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description?: React.ReactNode;
	action?: React.ReactNode;
	children: React.ReactNode;
}) {
	return (
		<section className="rounded-lg border border-foreground/10 bg-foreground/[0.02] p-5">
			<div className="flex items-start justify-between gap-4">
				<div className="min-w-0">
					<h2 className="text-base font-semibold text-foreground">
						{title}
					</h2>
					{description ? (
						<SmallText className="text-muted-foreground mt-1">
							{description}
						</SmallText>
					) : null}
				</div>
				{action}
			</div>
			<div className="mt-4">{children}</div>
		</section>
	);
}

function SetupSection({
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
	const agentInstructions = serializeAgentInstructions(
		clusterId,
		cloudEnv().VITE_APP_CLOUD_API_URL,
		data?.token,
	);
	return (
		<Section
			title="Setup"
			description="Run Rivet in your VPC, fully managed by Rivet."
		>
			<div className="flex flex-col gap-6 pt-3">
				<AgentPromptBanner
					code={agentInstructions ?? ""}
					containsSecret
					secretName="operator token"
					title="Use your coding agent"
					description="Paste instructions into your coding agent"
					buttonLabel="Copy agent instructions"
					buttonClassName="sm:w-56"
					disabled={!agentInstructions}
					isLoading={isLoading}
				/>
				<OrDivider label="or do it yourself" />
				<div className="w-full flex flex-col items-stretch justify-between gap-4 rounded-lg px-4 py-4 border border-border sm:flex-row sm:items-center">
					<div className="min-w-0">
						<p className="font-medium mb-1">
							Follow the setup guide
						</p>
						<p className="text-sm text-muted-foreground">
							Download the kit and cluster config to provision
							Rivet in your cloud.
						</p>
						<div className="mt-3 flex flex-wrap items-center gap-x-2 gap-y-2">
							<Button
								asChild
								variant="ghost"
								className="h-auto rounded-sm border-0 p-0 text-xs font-normal hover:bg-transparent hover:underline underline-offset-4"
							>
								<a href={BYOC_SETUP_KIT_URL} download>
									Download kit
								</a>
							</Button>
							<span
								aria-hidden="true"
								className="text-muted-foreground/40"
							>
								·
							</span>
							<Button
								variant="ghost"
								className="h-auto rounded-sm border-0 p-0 text-xs font-normal hover:bg-transparent hover:underline underline-offset-4"
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
							</Button>
							<span
								aria-hidden="true"
								className="text-muted-foreground/40"
							>
								·
							</span>
							<WithTooltip
								disabled={isLoading || !!data?.token}
								content={
									isError
										? "Could not load the operator token. Reload to retry or contact Enterprise Support."
										: "No active operator token. Contact Enterprise Support."
								}
								trigger={
									<span
										className="inline-flex"
										tabIndex={
											!isLoading && !data?.token
												? 0
												: undefined
										}
									>
										<CopyTrigger value={data?.token ?? ""}>
											<Button
												variant="ghost"
												className="h-auto rounded-sm border-0 p-0 text-xs font-normal hover:bg-transparent hover:underline underline-offset-4"
												aria-label="Copy operator token"
												isLoading={isLoading}
												disabled={!data?.token}
											>
												Copy token
											</Button>
										</CopyTrigger>
									</span>
								}
							/>
						</div>
					</div>
					<Button
						asChild
						variant="outline"
						className="w-full shrink-0 sm:w-56"
						endIcon={<Icon icon={faArrowRight} />}
					>
						<a
							href={BYOC_QUICKSTART_DOCS_URL}
							target="_blank"
							rel="noreferrer"
						>
							View setup guide
						</a>
					</Button>
				</div>
			</div>
		</Section>
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

function RegionsSection({ cluster }: { cluster: string }) {
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
		<Section
			title="Regions"
			description={
				<>
					Follow the{" "}
					<a
						href={BYOC_QUICKSTART_DOCS_URL}
						target="_blank"
						rel="noreferrer"
						className="underline"
					>
						setup guide
					</a>{" "}
					to add more regions.
				</>
			}
		>
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
		</Section>
	);
}

// Operator boot ids encode the boot timestamp as ms * 1024 plus a random component.
function bootIdToTimestamp(
	bootId: number | null | undefined,
): number | undefined {
	return bootId == null ? undefined : Math.floor(bootId / 1024);
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
				<Time value={bootIdToTimestamp(region.operatorBootId)} />
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

function CommandsSection({ cluster }: { cluster: string }) {
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
		<Section
			title="Commands"
			description="Commands are issued by Rivet Cloud to operate and upgrade your cluster."
			action={<RegionFilter cluster={cluster} />}
		>
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
		</Section>
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

function SupportCard({
	icon,
	title,
	subtitle,
	onClick,
	href,
}: {
	icon: typeof faEnvelope;
	title: string;
	subtitle: string;
	onClick?: () => void;
	href?: string;
}) {
	const body = (
		<>
			<Icon icon={icon} className="mt-0.5 shrink-0 text-foreground" />
			<span className="min-w-0">
				<span className="block font-semibold text-foreground">
					{title}
				</span>
				<span className="block truncate text-muted-foreground">
					{subtitle}
				</span>
			</span>
		</>
	);
	const className = cn(
		"flex flex-1 min-w-56 items-start gap-3 rounded-lg border border-foreground/10",
		"bg-foreground/[0.02] px-4 py-3 text-left text-sm transition-colors hover:bg-foreground/[0.05]",
	);

	if (href) {
		return (
			<a href={href} className={className}>
				{body}
			</a>
		);
	}

	return (
		<button type="button" onClick={onClick} className={className}>
			{body}
		</button>
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
			toast.error("Could not create the metrics token. Please try again.");
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
			toast.error("Could not revoke the metrics token. Please try again.");
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
		<Section
			title="Metrics ingest token"
			description="Authorizes sending OpenTelemetry metrics from this cluster to Rivet. Keep it secret."
		>
			{freshToken ? (
				<div className="flex flex-col gap-3">
					<div className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/[0.06] px-3 py-2 text-sm text-amber-600 dark:text-amber-400">
						<Icon
							icon={faTriangleExclamation}
							className="mt-0.5 shrink-0"
						/>
						<span>
							Copy this token now. For security it is only shown
							once and cannot be retrieved later.
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
						{`cloud_byocotel_${"•".repeat(16)}${data.tokenLastFour}`}
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
									onClick={() => setConfirmingRevoke(true)}
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
		</Section>
	);
}

function SupportSection() {
	return (
		<Section
			title="Enterprise Support"
			description="Every BYOC cluster includes enterprise support."
		>
			<div className="flex flex-wrap gap-2">
				<ByocContactTrigger>
					{(open) => (
						<SupportCard
							icon={faCalendarDays}
							title="Book a call"
							subtitle="Talk to the team about your cluster"
							onClick={open}
						/>
					)}
				</ByocContactTrigger>
				<ByocContactTrigger>
					{(open) => (
						<SupportCard
							icon={faSlack}
							title="Slack Connect"
							subtitle="Connect with the team on Slack"
							onClick={open}
						/>
					)}
				</ByocContactTrigger>
				<SupportCard
					icon={faEnvelope}
					title="Email support"
					subtitle={BYOC_SUPPORT_EMAIL}
					href={`mailto:${BYOC_SUPPORT_EMAIL}`}
				/>
			</div>
		</Section>
	);
}
