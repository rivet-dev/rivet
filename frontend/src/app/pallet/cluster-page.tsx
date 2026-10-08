import type { Rivet } from "@rivet-gg/cloud";
import {
	faAws,
	faClaude,
	faCopy,
	faCursor,
	faEye,
	faEyeSlash,
	faGlobe,
	faKey,
	faGoogleCloud,
	faLock,
	faMicrosoft,
	faOpenai,
	faSparkles,
	faTerminal,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";
import {
	Badge,
	Button,
	CopyTrigger,
	ClickToCopy,
	cn,
	DiscreteCopyButton,
	H1,
	ScrollArea,
	Skeleton,
	SmallText,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	WithTooltip,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { useDialog } from "@/app/use-dialog";
import { PALLET_CLOUDS, PALLET_NODE_SIZES } from "@/content/billing";
import { getMcpUrl } from "@/lib/env";
import { getStructuredApiErrorMessage, isRivetApiError } from "@/lib/errors";
import { serializeClusterAgentInstructions } from "./agent-instructions";

type Region = Rivet.v2.RegionsListResponse.Items.Item;
type WorkerPool = Rivet.v2.WorkerPoolsListResponse.Items.Item;
type Build = Rivet.v2.BuildsListResponse.Items.Item;
type Node = Rivet.v2.NodesListResponse.Items.Item;
type Status = { ready: boolean; deleting: boolean };

const CLOUD_ICONS = { aws: faAws, gcp: faGoogleCloud, azure: faMicrosoft };

export function PalletClusterPage({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const { data } = useQuery(
		dataProvider.palletClusterQueryOptions({ cluster }),
	);
	const regions = useQuery(
		dataProvider.palletClusterRegionsQueryOptions({ cluster }),
	);
	const primaryRegion = regions.data?.[0];

	return (
		<Frame>
			<header className="space-y-2">
				<div className="flex flex-wrap items-center gap-2">
					<H1 className="text-2xl truncate">
						{data?.cluster ?? cluster}
					</H1>
					{data ? <StatusBadge status={data.status} /> : null}
				</div>
				<div className="flex flex-wrap items-center gap-1.5">
					{regions.isLoading ? (
						<Skeleton className="h-5 w-40 rounded-full" />
					) : null}
					{primaryRegion ? (
						<Badge variant="outline" className="font-mono">
							{nodeSize(primaryRegion.node_size).label}
						</Badge>
					) : null}
					{regions.data?.map((region) => (
						<Badge
							key={region.region}
							variant="outline"
							className="gap-1.5"
						>
							<Icon icon={CLOUD_ICONS[region.cloud]} />
							{cloudLabel(region.cloud)} · {region.region}
						</Badge>
					))}
				</div>
			</header>

			<div className="flex justify-center">
				<AgentCallout
					cluster={cluster}
					region={primaryRegion}
					adminToken={data?.admin_token}
				/>
			</div>

			<ConnectionSection
				adminToken={data?.admin_token}
				regions={regions}
			/>

			<div className="space-y-8">
				<WorkerPoolsSection cluster={cluster} />
				<DeploymentsSection cluster={cluster} />
			</div>
		</Frame>
	);
}

PalletClusterPage.Skeleton = function PalletClusterPageSkeleton() {
	return (
		<Frame>
			<div className="space-y-2">
				<Skeleton className="h-8 w-64" />
				<Skeleton className="h-5 w-48 rounded-full" />
			</div>
			<div className="flex justify-center">
				<Skeleton className="h-11 w-80 rounded-full" />
			</div>
			<Section title="Connection information">
				<Panel className="divide-y divide-foreground/10">
					<SkeletonRow />
					<SkeletonRow />
					<SkeletonRow />
				</Panel>
			</Section>
			<div className="space-y-8">
				<Section title="Worker pools">
					<Panel className="overflow-hidden">
						<Table>
							<WorkerPoolsTableHeader />
							<TableBody>
								<LoadingRows columns={3} />
							</TableBody>
						</Table>
					</Panel>
				</Section>
				<Section title="Deployments">
					<Panel className="overflow-hidden">
						<Table>
							<DeploymentsTableHeader />
							<TableBody>
								<LoadingRows columns={4} />
							</TableBody>
						</Table>
					</Panel>
				</Section>
			</div>
		</Frame>
	);
};

export class ClusterLoadError extends Error {
	constructor(
		readonly cluster: string,
		options: { cause: unknown },
	) {
		super(`Could not load cluster ${cluster}.`, options);
	}
}

function describeClusterLoadError(cause: unknown) {
	const status = isRivetApiError(cause) ? cause.statusCode : undefined;
	const message = getStructuredApiErrorMessage(cause);
	if (status === 403) {
		return {
			title: "You don't have access to this cluster",
			description:
				message ??
				"Ask an admin of this organization to grant you access.",
			canRetry: false,
		};
	}
	return {
		title: "Couldn't load this cluster",
		description:
			message ??
			(status === undefined
				? "Couldn't reach the Cloud API. Check your connection and try again."
				: `The Cloud API returned an unexpected response (${status}).`),
		canRetry: true,
	};
}

export function PalletClusterError({ error }: { error: ClusterLoadError }) {
	const router = useRouter();
	const retry = useMutation({ mutationFn: () => router.invalidate() });
	const { title, description, canRetry } = describeClusterLoadError(
		error.cause,
	);

	return (
		<Frame>
			<H1 className="text-2xl truncate">{error.cluster}</H1>
			<Panel>
				<div className="flex flex-col items-center gap-3 px-6 py-14 text-center">
					<Icon
						icon={canRetry ? faTriangleExclamation : faLock}
						className="text-xl text-muted-foreground"
					/>
					<div className="max-w-md">
						<p className="font-medium">{title}</p>
						<SmallText className="mt-1 text-muted-foreground">
							{description}
						</SmallText>
					</div>
					{canRetry ? (
						<Button
							variant="outline"
							size="sm"
							isLoading={retry.isPending}
							onClick={() => retry.mutate()}
						>
							Retry
						</Button>
					) : null}
				</div>
			</Panel>
		</Frame>
	);
}

function Frame({ children }: { children: ReactNode }) {
	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto space-y-8">
					{children}
				</div>
			</ScrollArea>
		</div>
	);
}

function nodeSize(id: Region["node_size"]) {
	return (
		PALLET_NODE_SIZES.find((size) => size.id === id) ?? PALLET_NODE_SIZES[0]
	);
}

function cloudLabel(id: Region["cloud"]) {
	return PALLET_CLOUDS.find((cloud) => cloud.id === id)?.label ?? id;
}

function formatVcpu(vcpu: number) {
	return vcpu < 1 ? `1/${Math.round(1 / vcpu)}` : `${vcpu}`;
}

function statusLabel(status: Status) {
	if (status.deleting) return "Deleting";
	return status.ready ? "Running" : "Provisioning";
}

function statusDotClassName(status: Status) {
	if (status.deleting) return "bg-destructive";
	return status.ready ? "bg-emerald-500" : "bg-amber-500 animate-pulse";
}

function StatusDot({ className }: { className: string }) {
	return (
		<span
			aria-hidden="true"
			className={cn(
				"inline-block size-1.5 shrink-0 rounded-full",
				className,
			)}
		/>
	);
}

function StatusBadge({ status }: { status: Status }) {
	return (
		<Badge variant="outline" className="gap-1.5 font-medium">
			<StatusDot className={statusDotClassName(status)} />
			{statusLabel(status)}
		</Badge>
	);
}

function StatusText({ status }: { status: Status }) {
	return (
		<span className="inline-flex items-center gap-1.5">
			<StatusDot className={statusDotClassName(status)} />
			{statusLabel(status)}
		</span>
	);
}

function AgentCallout({
	cluster,
	region,
	adminToken,
}: {
	cluster: string;
	region: Region | undefined;
	adminToken: string | undefined;
}) {
	const { organization } = useCloudDataProvider();
	const instructions = serializeClusterAgentInstructions({
		cluster,
		mcpUrl: `${getMcpUrl()}?${new URLSearchParams({ organization })}`,
		publicEndpoint: region?.endpoints.public,
		privateEndpoint: region?.endpoints.private,
		adminToken,
	});

	return (
		<WithTooltip
			content="Copies setup instructions with your admin token. Paste only into your coding agent."
			trigger={
				<CopyTrigger value={instructions ?? ""}>
					<button
						type="button"
						disabled={!instructions}
						className="group inline-flex items-center gap-3 rounded-full border border-foreground/15 bg-foreground/[0.03] py-2 pl-4 pr-2 text-sm font-medium transition-colors hover:border-foreground/25 hover:bg-foreground/[0.06] disabled:cursor-not-allowed disabled:opacity-50"
					>
						<Icon icon={faSparkles} className="text-primary" />
						Connect your agent to Rivet
						<span className="flex items-center gap-1">
							{[faClaude, faOpenai, faCursor, faTerminal].map(
								(icon) => (
									<span
										key={icon.iconName}
										className="flex size-6 items-center justify-center rounded-full border border-foreground/10 bg-background text-xs text-muted-foreground"
									>
										<Icon icon={icon} />
									</span>
								),
							)}
							<span className="flex size-6 items-center justify-center text-xs text-muted-foreground transition-colors group-hover:text-foreground">
								<Icon icon={faCopy} />
							</span>
						</span>
					</button>
				</CopyTrigger>
			}
		/>
	);
}

function Section({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description?: string;
	action?: ReactNode;
	children: ReactNode;
}) {
	return (
		<section className="min-w-0 space-y-3">
			<div className="flex items-end justify-between gap-4">
				<div className="min-w-0">
					<h2 className="text-lg font-semibold text-foreground">
						{title}
					</h2>
					{description ? (
						<SmallText className="mt-0.5 text-muted-foreground">
							{description}
						</SmallText>
					) : null}
				</div>
				{action}
			</div>
			{children}
		</section>
	);
}

function Panel({
	className,
	children,
}: {
	className?: string;
	children: ReactNode;
}) {
	return (
		<div
			className={cn(
				"rounded-lg border border-foreground/10 bg-foreground/[0.02]",
				className,
			)}
		>
			{children}
		</div>
	);
}

function PanelMessage({
	children,
	onRetry,
}: {
	children: ReactNode;
	onRetry?: () => void;
}) {
	return (
		<div className="flex flex-col items-center gap-2 px-4 py-8 text-center">
			<SmallText className="text-muted-foreground">{children}</SmallText>
			{onRetry ? (
				<Button variant="outline" size="sm" onClick={onRetry}>
					Retry
				</Button>
			) : null}
		</div>
	);
}

function QueryError({
	what,
	error,
	onRetry,
}: {
	what: string;
	error: unknown;
	onRetry: () => void;
}) {
	return (
		<PanelMessage onRetry={onRetry}>
			{getStructuredApiErrorMessage(error) ?? `Couldn't load ${what}.`}
		</PanelMessage>
	);
}

function ConnectionSection({
	adminToken,
	regions,
}: {
	adminToken: string | undefined;
	regions: {
		data: Region[] | undefined;
		isLoading: boolean;
		isError: boolean;
		error: unknown;
		refetch: () => unknown;
	};
}) {
	const isMultiRegion = (regions.data?.length ?? 0) > 1;

	return (
		<Section title="Connection information">
			<Panel className="divide-y divide-foreground/10">
				{regions.isLoading ? (
					<>
						<SkeletonRow />
						<SkeletonRow />
						<SkeletonRow />
					</>
				) : null}
				{regions.isError ? (
					<QueryError
						what="connection information"
						error={regions.error}
						onRetry={() => void regions.refetch()}
					/>
				) : null}
				{regions.data?.length === 0 ? (
					<PanelMessage>
						This cluster has no regions yet.
					</PanelMessage>
				) : null}
				{regions.data?.flatMap((region) => [
					<PropertyRow
						key={`${region.region}-public`}
						icon={faGlobe}
						label="External endpoint"
						tag={isMultiRegion ? region.region : undefined}
						description={
							region.cidr_allowlist.length
								? "Public, filtered by the IP allowlist"
								: "Public"
						}
					>
						<CopyableValue value={region.endpoints.public} />
					</PropertyRow>,
					<PropertyRow
						key={`${region.region}-private`}
						icon={faLock}
						label="Internal endpoint"
						tag={isMultiRegion ? region.region : undefined}
						description="Private network, for workers in the VPC"
					>
						<CopyableValue value={region.endpoints.private} />
					</PropertyRow>,
				])}
				{regions.data?.length ? (
					<PropertyRow
						icon={faKey}
						label="Admin token"
						description="Full access. Use only in server-side code."
					>
						{adminToken ? (
							<CopyableValue value={adminToken} secret />
						) : (
							<Skeleton className="h-5 w-48" />
						)}
					</PropertyRow>
				) : null}
			</Panel>
		</Section>
	);
}

function PropertyRow({
	icon,
	label,
	tag,
	description,
	children,
}: {
	icon: typeof faGlobe;
	label: string;
	tag?: string;
	description: string;
	children: ReactNode;
}) {
	return (
		<div className="grid gap-2 px-5 py-4 md:grid-cols-[minmax(0,20rem)_minmax(0,1fr)] md:items-center md:gap-6">
			<div className="grid min-w-0 grid-cols-[1rem_minmax(0,1fr)] items-start gap-x-2 gap-y-0.5">
				<span className="flex h-5 items-center justify-center">
					<Icon
						icon={icon}
						className="text-sm text-muted-foreground"
					/>
				</span>
				<p className="flex items-center gap-2 text-sm font-medium">
					{label}
					{tag ? (
						<span className="font-mono text-xs font-normal text-muted-foreground">
							{tag}
						</span>
					) : null}
				</p>
				<p className="col-start-2 text-sm text-muted-foreground">
					{description}
				</p>
			</div>
			<div className="min-w-0 md:justify-self-end md:w-full md:max-w-lg">
				{children}
			</div>
		</div>
	);
}

function SkeletonRow() {
	return (
		<div className="grid gap-2 px-5 py-4 md:grid-cols-[minmax(0,20rem)_minmax(0,1fr)] md:items-center md:gap-6">
			<div className="grid grid-cols-[1rem_minmax(0,1fr)] gap-x-2 gap-y-0.5">
				<span className="flex h-5 items-center">
					<Skeleton className="size-3.5 rounded-sm" />
				</span>
				<span className="flex h-5 items-center">
					<Skeleton className="h-3.5 w-32" />
				</span>
				<span className="col-start-2 flex h-5 items-center">
					<Skeleton className="h-3.5 w-48" />
				</span>
			</div>
			<Skeleton className="h-[38px] w-full md:max-w-lg md:justify-self-end" />
		</div>
	);
}

function CopyableValue({ value, secret }: { value: string; secret?: boolean }) {
	const [isRevealed, setIsRevealed] = useState(false);
	const isMasked = secret && !isRevealed;

	return (
		<div className="flex items-center gap-1 rounded-md border border-foreground/10 bg-background py-1 pl-3 pr-1">
			<code className="min-w-0 flex-1 truncate font-mono text-sm">
				{isMasked ? "•".repeat(24) : value}
			</code>
			{secret ? (
				<WithTooltip
					content={isRevealed ? "Hide" : "Show"}
					trigger={
						<Button
							variant="ghost"
							size="icon-sm"
							aria-label={isRevealed ? "Hide" : "Show"}
							onClick={() =>
								setIsRevealed((revealed) => !revealed)
							}
						>
							<Icon icon={isRevealed ? faEyeSlash : faEye} />
						</Button>
					}
				/>
			) : null}
			<ClickToCopy value={value} delayDuration={0}>
				<Button variant="ghost" size="icon-sm" aria-label="Copy">
					<Icon icon={faCopy} />
				</Button>
			</ClickToCopy>
		</div>
	);
}

function DeploymentsSection({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const builds = useQuery(
		dataProvider.palletClusterBuildsQueryOptions({ cluster }),
	);
	const pools = useQuery(
		dataProvider.palletClusterWorkerPoolsQueryOptions({ cluster }),
	);
	const [deploy, setDeploy] = useState<{ buildId?: string } | null>(null);
	const canDeploy = !!builds.data?.length;
	const DeployDialog = useDialog.DeployPalletBuild.Dialog;

	return (
		<Section
			title="Deployments"
			action={
				<Button
					variant="outline"
					size="sm"
					disabled={!canDeploy}
					onClick={() => setDeploy({})}
				>
					Deploy
				</Button>
			}
		>
			<DeployDialog
				cluster={cluster}
				buildId={deploy?.buildId}
				dialogProps={{
					open: deploy !== null,
					onOpenChange: (open) => {
						if (!open) setDeploy(null);
					},
				}}
			/>
			<Panel className="overflow-hidden">
				{builds.isError ? (
					<QueryError
						what="deployments"
						error={builds.error}
						onRetry={() => void builds.refetch()}
					/>
				) : (
					<Table>
						<DeploymentsTableHeader />
						<TableBody>
							{builds.isLoading ? (
								<LoadingRows columns={4} />
							) : null}
							{builds.data?.map((build) => (
								<BuildRow
									key={build.build_id}
									build={build}
									pools={pools.data?.filter(
										(pool) =>
											pool.build_id === build.build_id,
									)}
									onDeploy={
										canDeploy
											? () =>
													setDeploy({
														buildId: build.build_id,
													})
											: undefined
									}
								/>
							))}
							{builds.data?.length === 0 ? (
								<EmptyRow columns={4}>
									No builds have been deployed yet.
								</EmptyRow>
							) : null}
						</TableBody>
					</Table>
				)}
			</Panel>
		</Section>
	);
}

function BuildRow({
	build,
	pools,
	onDeploy,
}: {
	build: Build;
	pools: WorkerPool[] | undefined;
	onDeploy: (() => void) | undefined;
}) {
	return (
		<TableRow>
			<TableCell>
				<WithTooltip
					content={build.build_id}
					trigger={
						<DiscreteCopyButton
							value={build.build_id}
							tooltip={false}
							size="sm"
							className="-my-1 -ml-2 h-7 px-2 font-mono text-xs font-normal text-foreground"
						>
							{build.build_id.slice(0, 8)}
						</DiscreteCopyButton>
					}
				/>
			</TableCell>
			<TableCell className="max-w-0">
				<DiscreteCopyButton
					value={build.image}
					size="sm"
					className="-my-1 -ml-2 h-7 px-2 font-mono text-xs font-normal text-foreground"
				>
					<span className="truncate">{build.image}</span>
				</DiscreteCopyButton>
			</TableCell>
			<TableCell className="text-xs">
				{pools === undefined ? (
					<Skeleton className="h-4 w-20" />
				) : pools.length ? (
					[...new Set(pools.map((pool) => pool.pool))].join(", ")
				) : (
					<span className="text-muted-foreground">Unassigned</span>
				)}
			</TableCell>
			<TableCell className="py-0 text-right">
				<Button
					variant="ghost"
					size="sm"
					className="-my-1 h-7 text-xs text-muted-foreground hover:text-foreground"
					disabled={!onDeploy}
					onClick={onDeploy}
				>
					Deploy
				</Button>
			</TableCell>
		</TableRow>
	);
}

function WorkerPoolsSection({ cluster }: { cluster: string }) {
	const dataProvider = useCloudDataProvider();
	const pools = useQuery(
		dataProvider.palletClusterWorkerPoolsQueryOptions({ cluster }),
	);
	const nodes = useQuery(
		dataProvider.palletClusterNodesQueryOptions({ cluster }),
	);

	return (
		<Section title="Worker pools">
			<Panel className="overflow-hidden">
				{pools.isError ? (
					<QueryError
						what="worker pools"
						error={pools.error}
						onRetry={() => void pools.refetch()}
					/>
				) : (
					<Table>
						<WorkerPoolsTableHeader />
						<TableBody>
							{pools.isLoading ? (
								<LoadingRows columns={3} />
							) : null}
							{pools.data?.map((pool) => (
								<WorkerPoolRow
									key={`${pool.namespace}/${pool.region}/${pool.pool}`}
									pool={pool}
									nodes={nodes.data?.filter(
										(node) =>
											node.namespace === pool.namespace &&
											node.region === pool.region &&
											node.pool === pool.pool,
									)}
								/>
							))}
							{pools.data?.length === 0 ? (
								<EmptyRow columns={3}>
									No worker pools yet.
								</EmptyRow>
							) : null}
						</TableBody>
					</Table>
				)}
			</Panel>
		</Section>
	);
}

function WorkerPoolRow({
	pool,
	nodes,
}: {
	pool: WorkerPool;
	nodes: Node[] | undefined;
}) {
	const size = nodeSize(pool.node_template.node_size);
	return (
		<TableRow>
			<TableCell>
				<p className="font-medium">{pool.pool}</p>
				<p className="text-xs text-muted-foreground">
					{pool.namespace} · {pool.region} · {formatVcpu(size.vcpu)}{" "}
					vCPU / {size.memoryGb} GB
				</p>
			</TableCell>
			<TableCell className="text-right text-xs tabular-nums">
				{nodes === undefined ? (
					<span className="text-muted-foreground">
						— / {pool.desired_count}
					</span>
				) : (
					<>
						{nodes.filter((node) => node.status.ready).length}
						<span className="text-muted-foreground">
							{" "}
							/ {pool.desired_count}
						</span>
					</>
				)}
			</TableCell>
			<TableCell className="text-xs">
				<StatusText status={pool.status} />
			</TableCell>
		</TableRow>
	);
}

function WorkerPoolsTableHeader() {
	return (
		<TableHeader>
			<TableRow>
				<ColumnHead>Pool</ColumnHead>
				<ColumnHead className="w-32 text-right">Workers</ColumnHead>
				<ColumnHead className="w-56">Status</ColumnHead>
			</TableRow>
		</TableHeader>
	);
}

function DeploymentsTableHeader() {
	return (
		<TableHeader>
			<TableRow>
				<ColumnHead className="w-32">Build</ColumnHead>
				<ColumnHead>Image</ColumnHead>
				<ColumnHead className="w-56">Used by</ColumnHead>
				<ColumnHead className="w-20">
					<span className="sr-only">Actions</span>
				</ColumnHead>
			</TableRow>
		</TableHeader>
	);
}

function ColumnHead({
	className,
	children,
}: {
	className?: string;
	children: ReactNode;
}) {
	return (
		<TableHead
			className={cn(
				"h-9 text-[11px] font-medium uppercase tracking-wide",
				className,
			)}
		>
			{children}
		</TableHead>
	);
}

function LoadingRows({ columns }: { columns: number }) {
	return (
		<>
			{[0, 1, 2].map((row) => (
				<TableRow key={row}>
					{Array.from({ length: columns }, (_, column) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: static placeholder cells
						<TableCell key={column}>
							<Skeleton className="h-4 w-full" />
						</TableCell>
					))}
				</TableRow>
			))}
		</>
	);
}

function EmptyRow({
	columns,
	children,
}: {
	columns: number;
	children: ReactNode;
}) {
	return (
		<TableRow>
			<TableCell
				colSpan={columns}
				className="py-8 text-center text-sm text-muted-foreground"
			>
				{children}
			</TableCell>
		</TableRow>
	);
}
