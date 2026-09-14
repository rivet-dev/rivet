import type { Rivet } from "@rivet-gg/cloud";
import {
	faArrowUpRightFromSquare,
	faCalendarDays,
	faChevronDown,
	faChevronRight,
	faDownload,
	faEnvelope,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { saveAs } from "file-saver";
import { useState } from "react";
import {
	Badge,
	Button,
	cn,
	H1,
	MultiSelectFormField,
	RelativeTime,
	ScrollArea,
	Skeleton,
	SmallText,
	WithTooltip,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import {
	BYOC_QUICKSTART_DOCS_URL,
	BYOC_SETUP_KIT_URL,
	BYOC_SUPPORT_EMAIL,
} from "@/content/byoc";
import { ByocContactTrigger } from "./byoc-contact-trigger";

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

	return (
		<Section
			title="Setup"
			description="Run Rivet in your VPC, fully managed by Rivet."
		>
			<div className="flex flex-wrap gap-2">
				<Button
					asChild
					variant="secondary"
					startIcon={<Icon icon={faDownload} />}
				>
					<a href={BYOC_SETUP_KIT_URL} download>
						Download setup kit
					</a>
				</Button>
				<WithTooltip
					disabled={isLoading || !!data}
					content={
						isError
							? "Could not load setup credentials. Reload this page to retry, or contact Enterprise Support below."
							: "No active operator token. Contact Enterprise Support below."
					}
					trigger={
						<span
							className="inline-flex"
							tabIndex={!isLoading && !data ? 0 : undefined}
						>
							<Button
								variant="secondary"
								startIcon={<Icon icon={faDownload} />}
								isLoading={isLoading}
								disabled={!clusterId || !data}
								onClick={() => {
									if (!clusterId || !data) return;
									saveAs(
										new Blob(
											[
												setupCredentials({
													clusterId,
													token: data.token,
												}),
											],
											{
												type: "application/json;charset=utf-8",
											},
										),
										"rivet-credentials.json",
									);
								}}
							>
								Download setup credentials
							</Button>
						</span>
					}
				/>
				<Button
					asChild
					variant="outline"
					endIcon={<Icon icon={faArrowUpRightFromSquare} />}
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
		</Section>
	);
}

function setupCredentials({
	clusterId,
	token,
}: {
	clusterId: string;
	token: string;
}) {
	return `${JSON.stringify(
		{ cloud_cluster_id: clusterId, operator_token: token },
		null,
		2,
	)}\n`;
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
