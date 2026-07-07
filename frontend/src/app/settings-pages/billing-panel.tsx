import { faArrowUpRight, faInfoCircle, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useMatch } from "@tanstack/react-router";
import { endOfMonth, startOfMonth } from "date-fns";
import { Suspense, useState } from "react";
import { BillingPlans } from "@/app/billing/billing-plans";
import { ComputeUsageCard } from "@/app/billing/compute-card";
import { billedMetricsMap, useBilledComputeCost } from "@/app/billing/hooks";
import { ManageBillingButton } from "@/app/billing/manage-billing-button";
import { formatMetricValue } from "@/app/billing/usage-card";
import {
	USAGE_METRICS,
	type UsageMetricConfig,
} from "@/app/billing/usage-metrics";
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
import { TwinklingSparkles } from "@/components/twinkling-sparkles";
import { features } from "@/lib/features";
import { ResourcePicker } from "./resource-picker";
import { SettingsCard } from "./settings-card";

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
	// Use `useQuery` (not `useSuspenseQuery`) so a slow billing fetch doesn't
	// bubble a Suspense to the route's pendingComponent and dim the top bar /
	// chrome while we wait. The backend returns the fully-computed breakdown.
	const { data: usage, isLoading } = useQuery(
		dataProvider.currentProjectBillingUsageQueryOptions(),
	);
	const [plansOpen, setPlansOpen] = useState(false);
	// Compute cost breakdown; already included in `usage.totalCents`.
	const compute = useBilledComputeCost();

	if (isLoading || !usage) {
		return <BillingSkeleton />;
	}

	const plan = usage.plan;
	const metricsByKey = billedMetricsMap(usage);
	const totalOverageCents = usage.totalCents;

	const periodStart = usage.currentPeriodStart
		? new Date(usage.currentPeriodStart)
		: startOfMonth(new Date());
	const periodEnd = usage.currentPeriodEnd
		? new Date(usage.currentPeriodEnd)
		: endOfMonth(new Date());

	return (
		<div className="space-y-8">
			<div className="grid grid-cols-1 md:grid-cols-2 gap-4">
				<CurrentPlanCard
					plan={plan}
					onUpgrade={() => setPlansOpen(true)}
				/>
				<CurrentBillCard
					total={totalOverageCents / 100}
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

				<SettingsCard divided>
					{USAGE_METRICS.map((metric, idx) => {
						const m = metricsByKey.get(metric.key);
						return (
							<UsageRow
								key={metric.key}
								metric={metric}
								current={BigInt(m?.usage ?? 0)}
								includedInPlan={
									m && m.included > 0
										? BigInt(m.included)
										: undefined
								}
								costCents={BigInt(m?.overageCents ?? 0)}
								last={idx === USAGE_METRICS.length - 1}
							/>
						);
					})}
				</SettingsCard>
			</div>

			{features.compute && !compute.isUnavailable ? (
				<ComputeUsageCard
					monthToDate={compute.monthToDate}
					isLoading={compute.isLoading}
					isError={compute.isError}
				/>
			) : null}
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
		<SettingsCard>
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
						<Icon icon={faArrowUpRight} className="size-3" />
					</span>
				</ManageBillingButton>
			</div>
		</SettingsCard>
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
	const elapsedMs = Math.max(
		0,
		Math.min(totalMs, now - periodStart.getTime()),
	);
	const _pct = totalMs > 0 ? (elapsedMs / totalMs) * 100 : 0;
	const daysLeft = Math.max(
		0,
		Math.ceil((periodEnd.getTime() - now) / (24 * 60 * 60 * 1000)),
	);
	const fmtDate = (d: Date) =>
		d.toLocaleDateString(undefined, { month: "short", day: "numeric" });

	return (
		<SettingsCard>
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
			<div className="flex items-center justify-between text-xs">
				<span className="text-muted-foreground">
					{fmtDate(periodStart)} – {fmtDate(periodEnd)}
				</span>
				<span className="text-foreground">{daysLeft} days left</span>
			</div>
		</SettingsCard>
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
