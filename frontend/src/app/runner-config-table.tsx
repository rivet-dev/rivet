import {
	faAws,
	faEllipsisVertical,
	faGoogleCloud,
	faHetznerH,
	faNextjs,
	faPencil,
	faRailway,
	faRivet,
	faTrash,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import type { ReactNode } from "react";
import { useMemo } from "react";
import {
	Button,
	DiscreteCopyButton,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	Text,
	WithTooltip,
} from "@/components";
import { Badge } from "@/components/ui/badge";
import { deriveProviderFromMetadata } from "@/lib/data";
import type { RivetActorError } from "@/queries/types";
import { RunnerPoolErrorPopover } from "./runner-pool-error-popover";

interface RunnerConfigsTableProps {
	isLoading?: boolean;
	isError?: boolean;
	hasNextPage?: boolean;
	fetchNextPage?: () => void;
	configs: [string, Rivet.RunnerConfigsListResponseRunnerConfigsValue][];
	totalDatacenterCount?: number;
	renderRegion: (regionId: string, opts: { abbreviated?: boolean }) => ReactNode;
	onEditConfig: (name: string) => void;
	onDeleteConfig: (name: string) => void;
}

export function RunnerConfigsTable({
	isLoading,
	isError,
	hasNextPage,
	fetchNextPage,
	configs,
	totalDatacenterCount,
	renderRegion,
	onEditConfig,
	onDeleteConfig,
}: RunnerConfigsTableProps) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead className="w-8"></TableHead>
					<TableHead className="pl-8">Name</TableHead>
					<TableHead>Provider</TableHead>
					<TableHead>Endpoint</TableHead>
					<TableHead>Datacenter</TableHead>
					<TableHead></TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{!isLoading && !isError && configs?.length === 0 ? (
					<TableRow>
						<TableCell colSpan={6}>
							<Text className="text-center">
								There's no providers matching criteria.
							</Text>
						</TableCell>
					</TableRow>
				) : null}
				{isError ? (
					<TableRow>
						<TableCell colSpan={6}>
							<Text className="text-center">
								An error occurred while fetching providers.
							</Text>
						</TableCell>
					</TableRow>
				) : null}
				{isLoading ? (
					<>
						<RowSkeleton />
						<RowSkeleton />
						<RowSkeleton />
						<RowSkeleton />
						<RowSkeleton />
						<RowSkeleton />
						<RowSkeleton />
						<RowSkeleton />
					</>
				) : null}
				{configs?.map(([id, config]) => (
					<Row
						name={id}
						{...config}
						totalDatacenterCount={totalDatacenterCount}
						renderRegion={renderRegion}
						onEditConfig={onEditConfig}
						onDeleteConfig={onDeleteConfig}
						key={id}
					/>
				))}

				{!isLoading && hasNextPage ? (
					<TableRow>
						<TableCell colSpan={6}>
							<Button
								variant="outline"
								isLoading={isLoading}
								onClick={() => fetchNextPage?.()}
								disabled={!hasNextPage}
							>
								Load more
							</Button>
						</TableCell>
					</TableRow>
				) : null}
			</TableBody>
		</Table>
	);
}

function RowSkeleton() {
	return (
		<TableRow>
			<TableCell className="w-8"></TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
		</TableRow>
	);
}

function Row({
	name,
	totalDatacenterCount,
	renderRegion,
	onEditConfig,
	onDeleteConfig,
	...value
}: {
	name: string;
	totalDatacenterCount?: number;
	renderRegion: (regionId: string, opts: { abbreviated?: boolean }) => ReactNode;
	onEditConfig: (name: string) => void;
	onDeleteConfig: (name: string) => void;
} & Rivet.RunnerConfigsListResponseRunnerConfigsValue) {
	const datacenters = Object.entries(value.datacenters);

	const isManaged = datacenters.some(
		([, config]) =>
			deriveProviderFromMetadata(config.metadata) === "rivet" ||
			"X-Rivet-Pool" in (config.serverless?.headers ?? {}),
	);

	return (
		<TableRow className="hover:bg-muted/50">
			<StatusCell
				datacenters={value.datacenters}
				renderRegion={renderRegion}
			/>
			<TableCell>
				<DiscreteCopyButton value={name}>{name}</DiscreteCopyButton>
			</TableCell>
			<TableCell>
				<ProviderSummary
					datacenters={datacenters}
					renderRegion={renderRegion}
				/>
			</TableCell>
			<TableCell>
				<Endpoints
					datacenters={datacenters}
					renderRegion={renderRegion}
				/>
			</TableCell>
			<TableCell>
				<Regions
					regions={Object.keys(value.datacenters)}
					totalDatacenterCount={totalDatacenterCount}
					renderRegion={renderRegion}
				/>
			</TableCell>
			<TableCell className="text-right">
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<Button variant="ghost" size="icon">
							<Icon icon={faEllipsisVertical} />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end">
						{isManaged ? null : (
							<DropdownMenuItem
								indicator={<Icon icon={faPencil} />}
								onSelect={() => onEditConfig(name)}
							>
								Edit
							</DropdownMenuItem>
						)}
						<DropdownMenuItem
							indicator={<Icon icon={faTrash} />}
							onSelect={() => onDeleteConfig(name)}
						>
							Delete
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			</TableCell>
		</TableRow>
	);
}

function StatusCell({
	datacenters,
	renderRegion,
}: {
	datacenters: Record<string, Rivet.RunnerConfigResponse>;
	renderRegion: (regionId: string, opts: { abbreviated?: boolean }) => ReactNode;
}) {
	const errors = useMemo(() => {
		const errorMap: Record<string, RivetActorError | undefined> = {};
		let hasErrors = false;
		for (const [dc, config] of Object.entries(datacenters)) {
			if (config.runnerPoolError) {
				errorMap[dc] = config.runnerPoolError;
				hasErrors = true;
			}
		}
		return hasErrors ? errorMap : null;
	}, [datacenters]);

	if (!errors) {
		return (
			<TableCell className="w-8">
				<span className="relative inline-flex size-5 items-center justify-center">
					<span className="absolute inline-flex size-3 rounded-full bg-green-400 opacity-75 animate-ping" />
					<span className="relative inline-flex size-2 rounded-full bg-green-500" />
				</span>
			</TableCell>
		);
	}

	return (
		<TableCell className="w-8">
			<RunnerPoolErrorPopover
				iconOnly
				errors={errors}
				renderRegion={(regionId) =>
					renderRegion(regionId, { abbreviated: true })
				}
			/>
		</TableCell>
	);
}

const PROVIDER_LABELS: Record<string, string> = {
	vercel: "Vercel",
	"next-js": "Next.js",
	railway: "Railway",
	hetzner: "Hetzner",
	aws: "AWS ECS",
	gcp: "Google Cloud Run",
	"gcp-cloud-run": "Google Cloud Run",
	rivet: "Rivet",
};

function getProviderLabel(provider: string | undefined): string {
	if (!provider) return "Unknown";
	return PROVIDER_LABELS[provider] ?? provider;
}

type RunnerKind = "serverless" | "runner";

function getDatacenterKind(
	config: Rivet.RunnerConfigResponse,
): RunnerKind {
	return config.serverless ? "serverless" : "runner";
}

function ProviderSummary({
	datacenters,
	renderRegion,
}: {
	datacenters: [string, Rivet.RunnerConfigResponse][];
	renderRegion: (regionId: string, opts: { abbreviated?: boolean }) => ReactNode;
}) {
	const breakdown = useMemo(() => {
		const rows = datacenters.map(([dc, config]) => ({
			dc,
			provider: deriveProviderFromMetadata(config.metadata) || "unknown",
			kind: getDatacenterKind(config),
		}));
		const providers = new Set(rows.map((r) => r.provider));
		const kinds = new Set(rows.map((r) => r.kind));
		return { rows, providers, kinds };
	}, [datacenters]);

	if (breakdown.rows.length === 0) return null;

	const isUniform =
		breakdown.providers.size === 1 && breakdown.kinds.size === 1;

	if (isUniform) {
		const kind = breakdown.kinds.values().next().value as RunnerKind;
		return (
			<div className="flex items-center gap-2">
				<Provider metadata={datacenters[0][1].metadata} />
				<Badge variant="outline" className="font-normal capitalize">
					{kind}
				</Badge>
			</div>
		);
	}

	const providerNode =
		breakdown.providers.size === 1 ? (
			<Provider metadata={datacenters[0][1].metadata} />
		) : (
			<span>Multiple</span>
		);

	const kindLabel =
		breakdown.kinds.size === 1
			? (breakdown.kinds.values().next().value as RunnerKind)
			: "Mixed";

	const showProviderInTooltip = breakdown.providers.size > 1;
	const showKindInTooltip = breakdown.kinds.size > 1;

	return (
		<WithTooltip
			content={
				<ul className="space-y-1">
					{breakdown.rows.map(({ dc, provider, kind }) => (
						<li key={dc} className="flex items-center gap-2">
							{renderRegion(dc, { abbreviated: false })}
							{showProviderInTooltip ? (
								<>
									<span className="text-muted-foreground">·</span>
									<ProviderInline provider={provider} />
								</>
							) : null}
							{showKindInTooltip ? (
								<>
									<span className="text-muted-foreground">·</span>
									<span className="capitalize">{kind}</span>
								</>
							) : null}
						</li>
					))}
				</ul>
			}
			trigger={
				<div className="flex items-center gap-2 cursor-default">
					{providerNode}
					<Badge variant="outline" className="font-normal capitalize">
						{kindLabel}
					</Badge>
				</div>
			}
		/>
	);
}

function truncateEndpoint(endpoint: string): string {
	if (endpoint.length > 32) {
		return `${endpoint.slice(0, 16)}...${endpoint.slice(-16)}`;
	}
	return endpoint;
}

function Endpoints({
	datacenters,
	renderRegion,
}: {
	datacenters: [string, Rivet.RunnerConfigResponse][];
	renderRegion: (regionId: string, opts: { abbreviated?: boolean }) => ReactNode;
}) {
	const perDatacenter = useMemo(() => {
		return datacenters
			.filter(([, config]) => config.serverless?.url)
			.map(([dc, config]) => [dc, config.serverless!.url] as const);
	}, [datacenters]);

	const uniqueEndpoints = useMemo(
		() => new Set(perDatacenter.map(([, url]) => url)),
		[perDatacenter],
	);

	if (perDatacenter.length === 0) {
		return (
			<span className="inline-flex h-9 items-center px-4 text-muted-foreground">
				—
			</span>
		);
	}

	if (uniqueEndpoints.size === 1) {
		const endpoint = perDatacenter[0][1];
		return (
			<DiscreteCopyButton value={endpoint}>
				{truncateEndpoint(endpoint)}
			</DiscreteCopyButton>
		);
	}

	return (
		<WithTooltip
			content={
				<ul className="space-y-1">
					{perDatacenter.map(([dc, endpoint]) => (
						<li key={dc} className="flex items-center gap-2">
							{renderRegion(dc, { abbreviated: false })}
							<span className="text-muted-foreground">·</span>
							<DiscreteCopyButton
								tooltip={false}
								value={endpoint}
								className="px-2 -mx-2"
							>
								<span>{truncateEndpoint(endpoint)}</span>
							</DiscreteCopyButton>
						</li>
					))}
				</ul>
			}
			trigger={
				<span className="inline-flex h-9 items-center px-4 cursor-default">
					Multiple endpoints
				</span>
			}
		/>
	);
}

const PROVIDER_ICONS: Record<string, typeof faVercel> = {
	vercel: faVercel,
	"next-js": faNextjs,
	railway: faRailway,
	hetzner: faHetznerH,
	aws: faAws,
	gcp: faGoogleCloud,
	"gcp-cloud-run": faGoogleCloud,
	rivet: faRivet,
};

function ProviderInline({ provider }: { provider: string | undefined }) {
	const icon = provider ? PROVIDER_ICONS[provider] : undefined;
	const label = getProviderLabel(provider);

	if (!icon) {
		return <span className="whitespace-nowrap">{label}</span>;
	}

	return (
		<span className="whitespace-nowrap">
			<Icon icon={icon} className="mr-1" />
			{label}
		</span>
	);
}

function Provider({ metadata }: { metadata: unknown }) {
	return <ProviderInline provider={deriveProviderFromMetadata(metadata)} />;
}

function Regions({
	regions,
	totalDatacenterCount,
	renderRegion,
}: {
	regions: string[];
	totalDatacenterCount?: number;
	renderRegion: (regionId: string, opts: { abbreviated?: boolean }) => ReactNode;
}) {
	if (
		totalDatacenterCount !== undefined &&
		regions.length === totalDatacenterCount
	) {
		return <span>Global</span>;
	}

	if (regions.length === 1) {
		return <>{renderRegion(regions[0], {})}</>;
	}

	return (
		<WithTooltip
			content={
				<ul className="space-y-1">
					{regions.map((region) => (
						<li key={region}>{renderRegion(region, {})}</li>
					))}
				</ul>
			}
			trigger={
				<span className="w-full cursor-pointer">Multiple regions</span>
			}
		/>
	);
}
