import {
	faArrowUpRight,
	faBarcodeRead,
	faDatabase,
	faInfoCircle,
	faPencil,
	faRunning,
	faSignalStream,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { TwinklingSparkles } from "@/components/twinkling-sparkles";
import { useQuery } from "@tanstack/react-query";
import { useMatch } from "@tanstack/react-router";
import { endOfMonth, startOfMonth } from "date-fns";
import { Suspense, useState } from "react";
import { BillingPlans } from "@/app/billing/billing-plans";
import { useBilledMetrics } from "@/app/billing/hooks";
import { ManageBillingButton } from "@/app/billing/manage-billing-button";
import {
	formatMetricValue,
	type MetricType,
} from "@/app/billing/usage-card";
import {
	Button,
	cn,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	formatCurrency,
	Skeleton,
	WithTooltip,
} from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { BILLING } from "@/content/billing";
import { ResourcePicker } from "./resource-picker";

type BilledMetric = keyof typeof BILLING.prices;

interface UsageMetricConfig {
	key: BilledMetric;
	title: string;
	description: string;
	icon: IconProp;
	metricType: MetricType;
}

const USAGE_METRICS: UsageMetricConfig[] = [
	{
		key: "actor_awake",
		title: "Awake actors",
		description: "Time your actors spend running and processing requests.",
		icon: faRunning,
		metricType: "hours",
	},
	{
		key: "kv_storage_used",
		title: "State storage",
		description: "Persistent data stored in actor state.",
		icon: faDatabase,
		metricType: "bytes",
	},
	{
		key: "kv_read",
		title: "Reads",
		description: "Data read from actor state, in 4KiB units.",
		icon: faBarcodeRead,
		metricType: "operations",
	},
	{
		key: "kv_write",
		title: "Writes",
		description: "Data written to actor state, in 4KiB units.",
		icon: faPencil,
		metricType: "operations",
	},
	{
		key: "gateway_egress",
		title: "Egress",
		description: "Network traffic sent from actors to clients.",
		icon: faSignalStream,
		metricType: "bytes",
	},
];

const PLAN_LABEL: Record<string, string> = {
	free: "Free",
	pro: "Hobby",
	team: "Team",
	enterprise: "Enterprise",
};

const PLAN_PRICE: Record<string, string> = {
	free: "$0/mo",
	pro: "$20/mo",
	team: "$200/mo",
	enterprise: "Custom",
};

const PLAN_BLURB: Record<string, string> = {
	free: "Perfect for exploring Rivet. Upgrade anytime to unlock more capacity and support.",
	pro: "For solo builders and hobby projects.",
	team: "For teams shipping production workloads.",
	enterprise: "Dedicated infrastructure and support.",
};

function calculateOverageCost(
	usage: bigint,
	includedInPlan: bigint | undefined,
	pricePerBillionUnits: bigint,
): bigint {
	if (!includedInPlan) return 0n;
	const overage = usage > includedInPlan ? usage - includedInPlan : 0n;
	return (overage * pricePerBillionUnits) / 1_000_000_000n;
}

export function BillingPanel() {
	// Use `useMatch` with `shouldThrow: false` instead of `useMatchRoute` so we
	// only render the project-scoped body when the project route is genuinely
	// in the active match tree (not just "intended" during a transition).
	const projectMatch = useMatch({
		from: "/_context/orgs/$organization/projects/$project",
		shouldThrow: false,
	});

	if (!projectMatch) {
		return (
			<ResourcePicker
				title="Pick a project"
				description="Billing is scoped to a project. Choose one to see usage and plan details."
				settings="billing"
				target="project"
			/>
		);
	}

	// During navigation from the resource picker, the project match enters
	// the tree before its loader resolves. `useCloudProjectDataProvider`
	// reads `useLoaderData(...).dataProvider`, which crashes when loader
	// data is undefined. Hold the skeleton until loader data lands.
	if (!projectMatch.loaderData) {
		return <BillingSkeleton />;
	}

	// Wrap in a local Suspense so any suspended descendant query (e.g.
	// `BillingPlans` via `useSuspenseQuery`) is caught here and renders
	// a local skeleton instead of bubbling to the project route's
	// `pendingComponent: FullscreenLoading`, which dims the entire chrome.
	return (
		<Suspense fallback={<BillingSkeleton />}>
			<BillingDrawerBody />
		</Suspense>
	);
}

function BillingDrawerBody() {
	const dataProvider = useCloudProjectDataProvider();
	// Use `useQuery` (not `useSuspenseQuery`) so a slow billing-details fetch
	// doesn't bubble a Suspense to the route's pendingComponent and dim the
	// top bar / chrome while we wait.
	const { data, isLoading } = useQuery(
		dataProvider.currentProjectBillingDetailsQueryOptions(),
	);
	const metrics = useBilledMetrics();
	const plan = data?.billing.activePlan || "free";
	const planIncluded = BILLING.included[plan] ?? BILLING.included.free;
	const [plansOpen, setPlansOpen] = useState(false);

	if (isLoading || !data) {
		return <BillingSkeleton />;
	}

	const totalOverageCents = USAGE_METRICS.reduce(
		(total, { key }) => {
			const current = metrics[key] || 0n;
			const includedInPlan = planIncluded[key];
			return (
				total +
				calculateOverageCost(
					current,
					includedInPlan,
					BILLING.prices[key],
				)
			);
		},
		0n,
	);

	const periodStart = data.billing.currentPeriodStart
		? new Date(data.billing.currentPeriodStart)
		: startOfMonth(new Date());
	const periodEnd = data.billing.currentPeriodEnd
		? new Date(data.billing.currentPeriodEnd)
		: endOfMonth(new Date());

	return (
		<div className="space-y-8">
			<div className="grid grid-cols-1 md:grid-cols-2 gap-4">
				<CurrentPlanCard
					plan={plan}
					onUpgrade={() => setPlansOpen(true)}
				/>
				<CurrentBillCard
					total={Number(totalOverageCents) / 100}
					periodStart={periodStart}
					periodEnd={periodEnd}
				/>
			</div>

			<Dialog open={plansOpen} onOpenChange={setPlansOpen}>
				<DialogContent className="max-w-4xl">
					<DialogHeader>
						<DialogTitle>Choose a plan</DialogTitle>
						<DialogDescription>
							Upgrade to unlock more capacity, support, and team
							features.
						</DialogDescription>
					</DialogHeader>
					<Suspense
						fallback={
							<Skeleton className="w-full h-64 rounded-lg" />
						}
					>
						<BillingPlans />
					</Suspense>
				</DialogContent>
			</Dialog>

			<div>
				<div className="flex items-end justify-between mb-4">
					<div>
						<h3 className="text-sm font-semibold text-foreground">
							Usage
						</h3>
						<p className="text-xs text-muted-foreground mt-0.5">
							Current billing period usage vs. plan limits.
						</p>
					</div>
					<p className="text-[11px] text-muted-foreground">
						Updated just now
					</p>
				</div>

				<div className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
					{USAGE_METRICS.map((metric, idx) => {
						const current = metrics[metric.key] || 0n;
						const includedInPlan = planIncluded[metric.key];
						const cost = calculateOverageCost(
							current,
							includedInPlan,
							BILLING.prices[metric.key],
						);
						return (
							<UsageRow
								key={metric.key}
								metric={metric}
								current={current}
								includedInPlan={includedInPlan}
								costCents={cost}
								last={idx === USAGE_METRICS.length - 1}
							/>
						);
					})}
				</div>
			</div>
		</div>
	);
}

function CurrentPlanCard({
	plan,
	onUpgrade,
}: {
	plan: string;
	onUpgrade: () => void;
}) {
	const label = PLAN_LABEL[plan] ?? "Free";
	const price = PLAN_PRICE[plan] ?? "$0/mo";
	const blurb = PLAN_BLURB[plan] ?? PLAN_BLURB.free;
	return (
		<div className="rounded-lg border border-foreground/10 bg-card p-5">
			<div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground mb-2">
				Current plan
			</div>
			<div className="flex items-baseline gap-2 mb-2">
				<span className="text-xl font-semibold text-foreground">
					{label}
				</span>
				<span className="text-xs text-muted-foreground">{price}</span>
			</div>
			<p className="text-xs text-muted-foreground mb-4 leading-relaxed">
				{blurb}
			</p>
			<div className="flex items-center gap-3">
				<Button
					variant="default"
					size="sm"
					startIcon={<TwinklingSparkles />}
					onClick={onUpgrade}
				>
					Upgrade plan
				</Button>
				<ManageBillingButton variant="ghost" size="sm">
					<span className="inline-flex items-center gap-1">
						Manage billing
						<Icon
							icon={faArrowUpRight}
							className="size-3"
						/>
					</span>
				</ManageBillingButton>
			</div>
		</div>
	);
}

function CurrentBillCard({
	total,
	periodStart,
	periodEnd,
}: {
	total: number;
	periodStart: Date;
	periodEnd: Date;
}) {
	const now = Date.now();
	const totalMs = periodEnd.getTime() - periodStart.getTime();
	const elapsedMs = Math.max(0, Math.min(totalMs, now - periodStart.getTime()));
	const pct = totalMs > 0 ? (elapsedMs / totalMs) * 100 : 0;
	const daysLeft = Math.max(
		0,
		Math.ceil((periodEnd.getTime() - now) / (24 * 60 * 60 * 1000)),
	);
	const fmtDate = (d: Date) =>
		d.toLocaleDateString(undefined, { month: "short", day: "numeric" });

	return (
		<div className="rounded-lg border border-foreground/10 bg-card p-5">
			<div className="flex items-center gap-1.5 mb-2">
				<span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
					Current bill
				</span>
				<WithTooltip
					delayDuration={0}
					trigger={
						<Icon
							icon={faInfoCircle}
							className="size-3 text-muted-foreground/60"
						/>
					}
					content="Total overage charges for the current billing period."
				/>
			</div>
			<div className="text-2xl font-semibold text-foreground mb-4">
				{formatCurrency(total)}
			</div>
			<div className="flex items-center justify-between text-xs mb-1.5">
				<span className="text-muted-foreground">Billing period</span>
				<span className="text-foreground">{daysLeft} days left</span>
			</div>
			<div className="relative h-1.5 rounded-full bg-foreground/10">
				<div
					className="absolute h-1.5 rounded-full bg-primary"
					style={{ width: `${pct}%` }}
				/>
			</div>
			<div className="flex items-center justify-between text-[11px] text-muted-foreground mt-1.5">
				<span>{fmtDate(periodStart)}</span>
				<span>{fmtDate(periodEnd)}</span>
			</div>
		</div>
	);
}

function UsageRow({
	metric,
	current,
	includedInPlan,
	costCents,
	last,
}: {
	metric: UsageMetricConfig;
	current: bigint;
	includedInPlan: bigint | undefined;
	costCents: bigint;
	last: boolean;
}) {
	const includedLabel = includedInPlan
		? `of ${formatMetricValue(includedInPlan, metric.metricType)}`
		: null;
	const currentLabel = formatMetricValue(current, metric.metricType);
	const cost = Number(costCents) / 100;

	const pct = includedInPlan
		? Math.min(100, Number((current * 100n) / includedInPlan))
		: 0;

	return (
		<div
			className={cn(
				"grid grid-cols-[2fr_1fr_1fr_auto] items-center gap-6 px-5 py-3.5",
				!last && "border-b border-foreground/10",
			)}
		>
			<div className="flex items-start gap-3 min-w-0">
				<div className="flex size-7 items-center justify-center rounded-md border border-foreground/10 mt-0.5 shrink-0">
					<Icon icon={metric.icon} className="size-3.5" />
				</div>
				<div className="min-w-0">
					<div className="text-sm font-medium text-foreground">
						{metric.title}
					</div>
					<div className="text-xs text-muted-foreground truncate">
						{metric.description}
					</div>
				</div>
			</div>
			<div className="text-sm tabular-nums text-foreground">
				{currentLabel}
			</div>
			<div className="min-w-0">
				<div className="text-xs text-muted-foreground">
					{includedLabel ?? "—"}
				</div>
				{includedInPlan ? (
					<div className="relative h-1 rounded-full bg-foreground/10 mt-1">
						<div
							className="absolute h-1 rounded-full bg-primary"
							style={{ width: `${pct}%` }}
						/>
					</div>
				) : null}
			</div>
			<div className="text-right">
				<div className="text-sm tabular-nums font-medium text-foreground">
					{formatCurrency(cost)}
				</div>
				<div className="text-[11px] text-muted-foreground">
					this period
				</div>
			</div>
		</div>
	);
}

function BillingSkeleton() {
	return (
		<div className="space-y-4">
			<div className="grid grid-cols-2 gap-4">
				<Skeleton className="w-full h-40 rounded-lg" />
				<Skeleton className="w-full h-40 rounded-lg" />
			</div>
			<Skeleton className="w-full h-64 rounded-lg" />
		</div>
	);
}
