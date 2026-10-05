import {
	faBucket,
	faBug,
	faCopy,
	faGlobe,
	faKey,
	faLock,
	faServer,
	faSpinnerThird,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";
import { CopyTrigger, DiscreteInput } from "@/components/copy-area";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { cn } from "@/components/lib/utils";
import { ClusterBillingPanel } from "./billing-panel";
import { AgentConnect } from "./agent-connect";
import { type Build, BUILDS, type TierId } from "./catalog";
import { CLOUD_ICONS } from "./create-cluster-page";
import { ClusterSidebar, type ClusterView } from "./cluster-sidebar";
import { DeploymentMap } from "./deployment-map";
import { IpAllowlist } from "./ip-allowlist";
import { LogsPanel } from "./logs-panel";
import { MockTopBar } from "./mock-top-bar";
import {
	type ClusterEndpoints,
	type ClusterSelection,
	clusterEndpoints,
	clusterState,
	detectUseCase,
	ENGINE_VERSION,
	nextEngineVersion,
	getCloud,
	getSize,
	getTier,
	getUseCase,
	quote,
	type Rollout,
	type RunnerConfig,
} from "./model";
import { RunnersConfig } from "./runners-config";

interface ClusterDashboardPageProps {
	selection: ClusterSelection;
	/** When the cluster was created; drives the mock provisioning timeline. */
	createdAt: number;
	/** When each extra runner was added, in ms since `createdAt`. */
	runnerAddedMs: number[];
	onAllowlistChange: (allowlist: string[]) => void;
	onTierChange: (tier: TierId) => void;
	onRunnersChange: (runners: RunnerConfig) => void;
	/**
	 * What the agent deployed, as the user described it to the agent. Drives
	 * the mock actors shown in Logs. Omit for a cluster nothing has been
	 * deployed to yet.
	 */
	deployedDescription?: string;
}

/** Hidden for now; flip to show the control plane upgrade simulator. */
const SHOW_DEBUG_CONTROLS = false;

export function ClusterDashboardPage({
	selection,
	createdAt,
	runnerAddedMs,
	onAllowlistChange,
	onTierChange,
	onRunnersChange,
	deployedDescription,
}: ClusterDashboardPageProps) {
	const detected = deployedDescription
		? detectUseCase(deployedDescription)
		: null;
	const deployed = detected !== null && detected.id !== "custom";
	const [view, setView] = useState<ClusterView>("overview");
	const [deployedBuildId, setDeployedBuildId] = useState(BUILDS[0].id);
	// Rolling updates shown on the deployment map: deploying a build rolls
	// the runners; changing plan rolls the control plane.
	const [rollouts, setRollouts] = useState<Rollout[]>([]);
	const startRollout = (kind: Rollout["kind"], version: string) =>
		setRollouts((prev) => [
			...prev,
			{ kind, version, startedMs: Date.now() - createdAt },
		]);
	const deploy = (id: string) => {
		const build = BUILDS.find((b) => b.id === id);
		if (!build) return;
		setDeployedBuildId(id);
		startRollout("runners", build.version);
	};
	// Debug control: Rivet upgrades the engine on the control plane out of
	// band, so the mock has a button to simulate one rolling through.
	const [engineVersion, setEngineVersion] = useState(ENGINE_VERSION);
	const changeTier = (next: TierId) => {
		onTierChange(next);
		startRollout("control-plane", engineVersion);
	};
	const upgradeControlPlane = () => {
		const next = nextEngineVersion(engineVersion);
		setEngineVersion(next);
		startRollout("control-plane", next);
	};
	// Nothing deployed yet: show the chat pattern as a preview.
	const useCase = deployed ? detected : getUseCase("chat");
	const endpoints = clusterEndpoints(selection);
	const q = quote(selection);
	const cloud = getCloud(selection.cloud);
	const tier = getTier(selection.tier);
	const region = selection.region.replace(/^(aws|gcp):/, "");

	// Stands in for polling the cluster's node list: nodes come up on a fixed
	// timeline after creation and polling stops once every node is ready.
	const nodesQuery = useQuery({
		queryKey: [
			"mock-cluster-nodes",
			selection.name,
			selection.tier,
			selection.runners,
			createdAt,
			runnerAddedMs,
			rollouts,
		],
		queryFn: () =>
			clusterState(
				selection,
				Date.now() - createdAt,
				runnerAddedMs,
				rollouts,
				BUILDS[0].version,
			),
		initialData: () =>
			clusterState(
				selection,
				Date.now() - createdAt,
				runnerAddedMs,
				rollouts,
				BUILDS[0].version,
			),
		refetchInterval: (query) =>
			query.state.data?.status === "active" ? false : 250,
	});
	const { nodes, status } = nodesQuery.data;

	const clusterHeader = (
		<header>
			<div>
				<div className="flex items-center gap-3">
					<h1 className="text-2xl font-semibold tracking-tight">
						{selection.name}
					</h1>
					{status === "active" ? (
						<Badge className="gap-1.5 border-emerald-500/40 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
							<span className="size-1.5 rounded-full bg-emerald-500" />
							Running
						</Badge>
					) : status === "updating" ? (
						<Badge className="gap-1.5 border-sky-500/40 bg-sky-500/10 text-sky-600 dark:text-sky-400">
							<Icon
								icon={faSpinnerThird}
								className="size-3 animate-spin"
							/>
							Updating
						</Badge>
					) : (
						<Badge className="gap-1.5 border-amber-500/40 bg-amber-500/10 text-amber-600 dark:text-amber-400">
							<Icon
								icon={faSpinnerThird}
								className="size-3 animate-spin"
							/>
							Provisioning
						</Badge>
					)}
				</div>
				<div className="mt-2 flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
					<PlanBadge plan={tier.id} />
					{cloud.id === "byoc" ? <PlanBadge plan="byoc" /> : null}
					<Badge variant="secondary" className="gap-1.5">
						<Icon icon={CLOUD_ICONS[cloud.id]} />
						{cloud.label} · {region}
					</Badge>
				</div>
			</div>
		</header>
	);

	// Same shell as the real `RouteLayout`: top bar, then a sidebar on the
	// left and `main` with the page content directly on the background.
	return (
		<div className="flex h-screen flex-col bg-background">
			<MockTopBar organization="Acme" cluster={selection.name} />
			<div className="flex min-h-0 flex-1">
				<ClusterSidebar view={view} onViewChange={setView} />
				<main className="flex min-h-0 min-w-0 flex-1 flex-col">
					{view === "logs" ? (
						<LogsPanel
							useCase={useCase}
							region={region}
							clusterName={selection.name}
							className="min-h-0 flex-1"
						/>
					) : null}

					{view === "overview" ? (
						<PageCard className="space-y-10">
							{clusterHeader}
							<AgentConnect />
							<Section title="Connection information">
								<ConnectionInfo
									endpoints={endpoints}
									includedObjectStorageGb={q.objectStorageGb}
								/>
							</Section>
							<Section
								title="Cluster map"
								aside={
									SHOW_DEBUG_CONTROLS ? (
										<Button
											variant="outline"
											size="sm"
											className="h-7 text-xs"
											startIcon={<Icon icon={faBug} />}
											disabled={status !== "active"}
											onClick={upgradeControlPlane}
										>
											Debug: upgrade control plane to{" "}
											{nextEngineVersion(engineVersion)}
										</Button>
									) : undefined
								}
							>
								<DeploymentMap
									nodes={nodes}
									externalHost={
										new URL(endpoints.external.url).host
									}
								/>
							</Section>
							<Section title="Builds">
								<BuildsTable
									builds={BUILDS}
									deployedId={deployedBuildId}
									onDeploy={deploy}
									disabled={status !== "active"}
								/>
							</Section>
						</PageCard>
					) : null}

					{view === "settings" ? (
						<PageCard className="space-y-10">
							<Section
								title="Compute"
								description="The control plane is sized by your plan. Runners run your actors; add nodes or pick a larger size and new nodes join within a minute."
							>
								<Card>
									<CardContent className="flex flex-col gap-6 pt-6">
										<div className="flex items-center justify-between gap-4 rounded-lg border border-border px-4 py-3 text-sm">
											<span className="flex items-center gap-3">
												<Icon
													icon={faServer}
													className="text-muted-foreground"
												/>
												<span className="flex flex-col">
													<span className="font-medium">
														Control plane
													</span>
													<span className="text-xs text-muted-foreground">
														Preconfigured by the{" "}
														{tier.label} plan.
														Upgrade in Billing to
														change.
													</span>
												</span>
											</span>
											<span className="font-mono text-muted-foreground">
												{tier.controlPlane.nodes} ×{" "}
												{
													getSize(
														tier.controlPlane.size,
													).label
												}
											</span>
										</div>
										<div className="flex flex-col gap-3">
											<span className="text-sm font-medium">
												Runners
											</span>
											<RunnersConfig
												tier={tier}
												cloud={cloud}
												value={selection.runners}
												onChange={onRunnersChange}
											/>
										</div>
									</CardContent>
								</Card>
							</Section>
							<Section
								title="Allowed inbound IP addresses"
								description="Applies to the external endpoint. Changes take effect within a few seconds and need no restart."
							>
								<Card>
									<CardContent className="pt-6">
										<IpAllowlist
											compact
											value={selection.allowlist}
											onChange={onAllowlistChange}
										/>
									</CardContent>
								</Card>
							</Section>
						</PageCard>
					) : null}

					{view === "billing" ? (
						<PageCard>
							<Section
								title="Billing"
								description="What this cluster costs: a fixed control plane from your plan plus runner compute priced per node for its cloud. Actor storage, object storage, and egress are included up to the plan's allowance."
							>
								<ClusterBillingPanel
									selection={selection}
									onTierChange={changeTier}
								/>
							</Section>
						</PageCard>
					) : null}
				</main>
			</div>
		</div>
	);
}

/** The outlined, scrollable card the page content sits in, matching the real dashboard. */
function PageCard({
	className,
	children,
}: {
	className?: string;
	children: ReactNode;
}) {
	return (
		<div className="my-2 mr-2 flex min-h-0 flex-1 overflow-hidden rounded-lg border bg-card">
			<ScrollArea className="h-full w-full">
				<div
					className={cn(
						"mx-auto w-full max-w-6xl px-6 py-6",
						className,
					)}
				>
					{children}
				</div>
			</ScrollArea>
		</div>
	);
}

/**
 * Aiven-style connection information: endpoints, the admin token, and object
 * storage credentials inline at the top of the overview, not behind a button.
 */
function ConnectionInfo({
	endpoints,
	includedObjectStorageGb,
}: {
	endpoints: ClusterEndpoints;
	includedObjectStorageGb: number;
}) {
	return (
		<Card>
			<CardContent className="grid gap-6 pt-6 lg:grid-cols-2">
				<div className="flex flex-col gap-5">
					<Endpoint
						icon={faGlobe}
						title="External endpoint"
						detail="Public. Filtered by the IP allowlist in Settings."
						url={endpoints.external.url}
					/>
					<Endpoint
						icon={faLock}
						title="Internal endpoint"
						detail="Private network only, for runners in the same VPC."
						url={endpoints.internal.url}
					/>
					<div className="flex flex-col gap-2">
						<EndpointTitle
							icon={faKey}
							title="Admin token"
							detail="Works on both endpoints."
						/>
						<DiscreteInput value={endpoints.adminToken} />
					</div>
				</div>
				<ObjectStorageEndpoint
					storage={endpoints.objectStorage}
					includedGb={includedObjectStorageGb}
				/>
			</CardContent>
		</Card>
	);
}

function EndpointTitle({
	icon,
	title,
	detail,
}: {
	icon: IconProp;
	title: string;
	detail: string;
}) {
	return (
		<div className="flex flex-col gap-0.5">
			<span className="flex items-center gap-2 text-sm font-semibold">
				<Icon icon={icon} className="text-muted-foreground" />
				{title}
			</span>
			<span className="text-xs text-muted-foreground">{detail}</span>
		</div>
	);
}

function Endpoint({
	icon,
	title,
	detail,
	url,
}: {
	icon: IconProp;
	title: string;
	detail: string;
	url: string;
}) {
	return (
		<div className="flex flex-col gap-2">
			<EndpointTitle icon={icon} title={title} detail={detail} />
			<CopyTrigger value={url}>
				<button
					type="button"
					className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/40 px-3 py-2 text-left font-mono text-xs hover:border-muted-foreground/40"
				>
					<span className="truncate">{url}</span>
					<Icon icon={faCopy} className="text-muted-foreground" />
				</button>
			</CopyTrigger>
		</div>
	);
}

function ObjectStorageEndpoint({
	storage,
	includedGb,
}: {
	storage: ClusterEndpoints["objectStorage"];
	includedGb: number;
}) {
	return (
		<div className="flex flex-col gap-2">
			<div className="flex flex-col gap-0.5">
				<span className="flex items-center gap-2 text-sm font-semibold">
					<Icon icon={faBucket} className="text-muted-foreground" />
					Object storage
				</span>
				<span className="text-xs text-muted-foreground">
					S3-compatible. {includedGb} GB included, for blobs and
					uploads that do not belong in actor state. Works with any S3
					SDK.
				</span>
			</div>
			<CopyTrigger value={storage.url}>
				<button
					type="button"
					className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/40 px-3 py-2 text-left font-mono text-xs hover:border-muted-foreground/40"
				>
					<span className="truncate">{storage.url}</span>
					<Icon icon={faCopy} className="text-muted-foreground" />
				</button>
			</CopyTrigger>
			<div className="grid gap-1.5 text-xs">
				<KeyValue label="Bucket">
					<CopyTrigger value={storage.bucket}>
						<button
							type="button"
							className="truncate font-mono hover:text-primary"
						>
							{storage.bucket}
						</button>
					</CopyTrigger>
					<span className="text-muted-foreground">
						{" "}
						· region {storage.region}
					</span>
				</KeyValue>
				<KeyValue label="Access key">
					<span className="font-mono">{storage.accessKeyId}</span>
				</KeyValue>
				<KeyValue label="Secret key">
					<DiscreteInput value={storage.secretAccessKey} />
				</KeyValue>
			</div>
		</div>
	);
}

function KeyValue({
	label,
	children,
}: {
	label: string;
	children: React.ReactNode;
}) {
	return (
		<div className="flex items-center gap-2">
			<span className="w-24 shrink-0 text-muted-foreground">{label}</span>
			<div className="min-w-0 flex-1 truncate">{children}</div>
		</div>
	);
}

function BuildsTable({
	builds,
	deployedId,
	onDeploy,
	disabled,
}: {
	builds: Build[];
	deployedId: string;
	onDeploy: (id: string) => void;
	/** One rollout at a time: deploys wait until the cluster is running. */
	disabled?: boolean;
}) {
	// Deploying replaces the live build, so it asks for confirmation first.
	const [pending, setPending] = useState<Build | null>(null);
	const current = builds.find((b) => b.id === deployedId);
	return (
		<Card>
			<Dialog
				open={pending !== null}
				onOpenChange={(open) => {
					if (!open) setPending(null);
				}}
			>
				<DialogContent className="max-w-md">
					<DialogHeader>
						<DialogTitle>Deploy {pending?.version}?</DialogTitle>
						<DialogDescription>
							{current
								? `Rolls out to runners one node at a time, replacing ${current.version}. Running actors are drained onto the new build; clients reconnect automatically.`
								: "Rolls out to runners one node at a time. Running actors are drained onto the new build; clients reconnect automatically."}
						</DialogDescription>
					</DialogHeader>
					{pending ? (
						<div className="rounded-lg border border-border bg-muted/40 px-3 py-2 text-xs">
							<div className="truncate">{pending.message}</div>
							<div className="font-mono text-[11px] text-muted-foreground">
								{pending.commit} · {pending.createdAgo}
							</div>
						</div>
					) : null}
					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setPending(null)}
						>
							Cancel
						</Button>
						<Button
							onClick={() => {
								if (pending) onDeploy(pending.id);
								setPending(null);
							}}
						>
							Deploy {pending?.version}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead>Version</TableHead>
						<TableHead>Build</TableHead>
						<TableHead>Created</TableHead>
						<TableHead className="text-right">Status</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{builds.map((b) => {
						const deployed = b.id === deployedId;
						return (
							<TableRow
								key={b.id}
								data-state={deployed ? "selected" : undefined}
							>
								<TableCell className="font-mono text-xs">
									{b.version}
								</TableCell>
								<TableCell className="max-w-96">
									<div className="truncate text-xs">
										{b.message}
									</div>
									<div className="font-mono text-[11px] text-muted-foreground">
										{b.commit}
									</div>
								</TableCell>
								<TableCell className="text-xs text-muted-foreground">
									{b.createdAgo}
								</TableCell>
								<TableCell className="text-right">
									{deployed ? (
										<Badge className="gap-1.5 border-emerald-500/40 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
											<span className="size-1.5 rounded-full bg-emerald-500" />
											Deployed
										</Badge>
									) : (
										<Button
											variant="outline"
											size="sm"
											className="h-7 text-xs"
											disabled={disabled}
											onClick={() => setPending(b)}
										>
											Deploy
										</Button>
									)}
								</TableCell>
							</TableRow>
						);
					})}
				</TableBody>
			</Table>
		</Card>
	);
}

function Section({
	title,
	description,
	aside,
	children,
}: {
	title: string;
	description?: string;
	/** Controls rendered to the right of the heading. */
	aside?: ReactNode;
	children: ReactNode;
}) {
	return (
		<section className="flex min-w-0 flex-col gap-4">
			<div className="flex flex-wrap items-end justify-between gap-3">
				<div className="min-w-0 flex-1">
					<h2 className="text-xl font-semibold">{title}</h2>
					{description ? (
						<p className="mt-1 max-w-3xl text-sm text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{aside !== undefined ? (
					<div className="flex shrink-0 items-center gap-2">
						{aside}
					</div>
				) : null}
			</div>
			{children}
		</section>
	);
}
