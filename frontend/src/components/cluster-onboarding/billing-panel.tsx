import {
	faArrowUpRight,
	faBucket,
	faHdd,
	faInfoCircle,
	faServer,
	faSignalStream,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { endOfMonth, startOfMonth } from "date-fns";
import { useState } from "react";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import { SettingsCard } from "@/app/settings-pages/settings-card";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { WithTooltip } from "@/components/ui/tooltip";
import { cn } from "@/components/lib/utils";
import { formatCurrency } from "@/components/lib/formatter";
import { TwinklingSparkles } from "@/components/twinkling-sparkles";
import { BILLING, type TierId, TIERS } from "./catalog";
import {
	type ClusterSelection,
	clampRunners,
	formatMemory,
	formatUsd,
	formatVcpu,
	type Quote,
	quote,
} from "./model";

interface ClusterBillingPanelProps {
	selection: ClusterSelection;
	onTierChange: (tier: TierId) => void;
}

/**
 * Same layout as the real `BillingPanel` (current plan + current bill, a
 * choose-a-plan dialog, and usage rows), but priced per cluster: the plan
 * fee covers a fixed control plane, runner compute is priced per node for
 * the chosen cloud, and storage, object storage, and egress are included up
 * to the plan's allowance.
 */
export function ClusterBillingPanel({
	selection,
	onTierChange,
}: ClusterBillingPanelProps) {
	const q = quote(selection);
	const usage = mockUsage(q);
	const [plansOpen, setPlansOpen] = useState(false);

	const periodStart = startOfMonth(new Date());
	const periodEnd = endOfMonth(new Date());
	const overageUsd = usage.reduce((sum, u) => sum + u.overageUsd, 0);

	return (
		<div className="space-y-8">
			<div className="grid grid-cols-1 gap-4 md:grid-cols-2">
				<CurrentPlanCard
					q={q}
					onChangePlan={() => setPlansOpen(true)}
				/>
				<CurrentBillCard
					subscriptionUsd={q.totalUsd}
					overageUsd={overageUsd}
					periodStart={periodStart}
					periodEnd={periodEnd}
				/>
			</div>

			<Dialog open={plansOpen} onOpenChange={setPlansOpen}>
				<DialogContent className="max-w-5xl">
					<DialogHeader>
						<DialogTitle>Choose a plan</DialogTitle>
						<DialogDescription>
							The plan sets the node count, support, and the
							largest node size. Node pricing depends on the cloud
							and size you picked for this cluster.
						</DialogDescription>
					</DialogHeader>
					<div className="grid grid-cols-1 gap-4 md:grid-cols-4">
						{TIERS.map((tier) => {
							const current = tier.id === selection.tier;
							const tierQuote = quote({
								...selection,
								tier: tier.id,
								runners: clampRunners(
									tier.id,
									selection.runners,
								),
							});
							return (
								<div
									key={tier.id}
									className={cn(
										"flex h-full flex-col rounded-lg border p-6 transition-colors hover:bg-secondary/20",
										current && "border-foreground",
									)}
								>
									<div className="mb-3">
										<PlanBadge
											plan={tier.id}
											className="text-sm"
										/>
									</div>
									<div className="mb-4 flex items-baseline gap-1">
										<span className="text-3xl font-medium tracking-[-0.015em]">
											{tier.monthlyUsd === 0
												? "$0"
												: formatUsd(tier.monthlyUsd)}
										</span>
										<span className="ml-1 text-xs text-muted-foreground">
											/mo + runners
										</span>
									</div>
									<div className="mb-4 h-px bg-border" />
									<p className="mb-4 min-h-10 text-sm leading-5 text-muted-foreground">
										{tier.description}
									</p>
									<dl className="flex-1 divide-y divide-border border-y text-xs">
										{tier.highlights.map((h) => (
											<div key={h} className="py-2">
												<dt>{h}</dt>
											</div>
										))}
										<div className="flex items-baseline justify-between gap-3 py-2">
											<dt className="text-muted-foreground">
												Control plane{" "}
												{tierQuote.controlPlane.nodes} ×{" "}
												{
													tierQuote.controlPlane.size
														.label
												}
											</dt>
											<dd className="whitespace-nowrap font-medium">
												Fixed
											</dd>
										</div>
										<div className="flex items-baseline justify-between gap-3 py-2">
											<dt className="text-muted-foreground">
												Runners{" "}
												{tierQuote.runners.count} ×{" "}
												{tierQuote.runners.size.label}
											</dt>
											<dd className="whitespace-nowrap font-medium">
												{formatUsd(tierQuote.totalUsd)}
												/mo total
											</dd>
										</div>
									</dl>
									<Button
										variant={
											current ? "secondary" : "default"
										}
										className="mt-6 w-full"
										disabled={current}
										onClick={() => {
											onTierChange(tier.id);
											setPlansOpen(false);
										}}
									>
										{current
											? "Current plan"
											: tier.id === "enterprise"
												? "Contact us"
												: `Switch to ${tier.label}`}
									</Button>
								</div>
							);
						})}
					</div>
				</DialogContent>
			</Dialog>

			<div>
				<div className="mb-4 flex items-end justify-between">
					<div>
						<h3 className="text-sm font-semibold text-foreground">
							Subscription
						</h3>
						<p className="mt-0.5 text-xs text-muted-foreground">
							Billed monthly for this cluster. Scale runners in
							Settings or change plan any time; changes are
							prorated.
						</p>
					</div>
				</div>
				<SettingsCard divided>
					{q.lines.map((line) => (
						<div
							key={line.label}
							className="flex items-center justify-between gap-6 border-b border-foreground/10 px-5 py-3 text-sm"
						>
							<span className="text-foreground">
								{line.label}
							</span>
							<span
								className={cn(
									"tabular-nums",
									typeof line.amount === "number"
										? "font-medium text-foreground"
										: "text-muted-foreground",
								)}
							>
								{typeof line.amount === "number"
									? formatCurrency(line.amount)
									: line.amount}
							</span>
						</div>
					))}
					<div className="flex items-center justify-between gap-6 px-5 py-3 text-sm">
						<span className="font-medium text-foreground">
							Total per month
						</span>
						<span className="font-semibold tabular-nums text-foreground">
							{formatCurrency(q.totalUsd)}
						</span>
					</div>
				</SettingsCard>
			</div>

			<div>
				<div className="mb-4 flex items-end justify-between">
					<div>
						<h3 className="text-sm font-semibold text-foreground">
							Usage
						</h3>
						<p className="mt-0.5 text-xs text-muted-foreground">
							Current billing period usage vs. what the cluster
							includes.
						</p>
					</div>
					<p className="text-[11px] text-muted-foreground">
						Updated just now
					</p>
				</div>
				<SettingsCard divided>
					<ControlPlaneRow q={q} />
					<RunnerComputeRow q={q} />
					{usage.map((u, idx) => (
						<UsageRow
							key={u.title}
							usage={u}
							last={idx === usage.length - 1}
						/>
					))}
				</SettingsCard>
			</div>
		</div>
	);
}

function CurrentPlanCard({
	q,
	onChangePlan,
}: {
	q: Quote;
	onChangePlan: () => void;
}) {
	return (
		<SettingsCard>
			<div className="mb-2 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
				Current plan
			</div>
			<div className="mb-2 flex items-center gap-2">
				<PlanBadge plan={q.tier.id} />
				{q.cloud.id === "byoc" ? <PlanBadge plan="byoc" /> : null}
				<span className="text-xs text-muted-foreground">
					{q.tier.monthlyUsd === 0
						? "$0/mo"
						: `${formatUsd(q.tier.monthlyUsd)}/mo`}{" "}
					control plane + {q.runners.count} × {q.runners.size.label}{" "}
					runners on {q.cloud.label}
				</span>
			</div>
			<p className="mb-4 text-xs leading-relaxed text-muted-foreground">
				{q.tier.description}
			</p>
			<div className="flex items-center gap-3">
				<Button
					variant="default"
					size="sm"
					startIcon={<TwinklingSparkles />}
					onClick={onChangePlan}
				>
					{q.tier.id === "enterprise"
						? "Change plan"
						: "Upgrade plan"}
				</Button>
				<Button variant="ghost" size="sm">
					<span className="inline-flex items-center gap-1">
						Manage billing
						<Icon icon={faArrowUpRight} className="size-3" />
					</span>
				</Button>
			</div>
		</SettingsCard>
	);
}

function CurrentBillCard({
	subscriptionUsd,
	overageUsd,
	periodStart,
	periodEnd,
}: {
	subscriptionUsd: number;
	overageUsd: number;
	periodStart: Date;
	periodEnd: Date;
}) {
	const now = Date.now();
	const daysLeft = Math.max(
		0,
		Math.ceil((periodEnd.getTime() - now) / (24 * 60 * 60 * 1000)),
	);
	const fmtDate = (d: Date) =>
		d.toLocaleDateString(undefined, { month: "short", day: "numeric" });

	return (
		<SettingsCard>
			<div className="mb-2 flex items-center gap-1.5">
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
					content="Subscription for this period plus any usage over what the cluster includes."
				/>
			</div>
			<div className="mb-1 text-2xl font-semibold text-foreground">
				{formatCurrency(subscriptionUsd + overageUsd)}
			</div>
			<p className="mb-4 text-xs text-muted-foreground">
				{formatCurrency(subscriptionUsd)} subscription +{" "}
				{formatCurrency(overageUsd)} usage
			</p>
			<div className="flex items-center justify-between text-xs">
				<span className="text-muted-foreground">
					{fmtDate(periodStart)} – {fmtDate(periodEnd)}
				</span>
				<span className="text-foreground">{daysLeft} days left</span>
			</div>
		</SettingsCard>
	);
}

interface UsageLine {
	title: string;
	description: string;
	icon: IconProp;
	usedGb: number;
	includedGb: number;
	usdPerGb: number;
	overageUsd: number;
}

// Deterministic month-to-date usage as fractions of the cluster's allowance.
// Egress runs over so the overage path is visible in the mock.
function mockUsage(q: Quote): UsageLine[] {
	const line = (
		title: string,
		description: string,
		icon: IconProp,
		includedGb: number,
		fraction: number,
		usdPerGb: number,
	): UsageLine => {
		const usedGb = Math.round(includedGb * fraction * 10) / 10;
		const overGb = Math.max(0, usedGb - includedGb);
		return {
			title,
			description,
			icon,
			usedGb,
			includedGb,
			usdPerGb,
			overageUsd: Math.round(overGb * usdPerGb * 100) / 100,
		};
	};
	return [
		line(
			"Actor storage",
			`SQLite state for every actor in this cluster. ${formatUsd(BILLING.extraStorageUsdPerGb)}/GB over.`,
			faHdd,
			q.storageGb,
			0.37,
			BILLING.extraStorageUsdPerGb,
		),
		line(
			"Object storage",
			`S3-compatible buckets. ${formatUsd(BILLING.extraObjectStorageUsdPerGb)}/GB over.`,
			faBucket,
			q.objectStorageGb,
			0.42,
			BILLING.extraObjectStorageUsdPerGb,
		),
		line(
			"Egress",
			`Traffic from your actors to clients. ${formatUsd(BILLING.egressUsdPerGb)}/GB over.`,
			faSignalStream,
			q.egressGb,
			1.28,
			BILLING.egressUsdPerGb,
		),
	];
}

const ROW_CLASS =
	"grid grid-cols-[2fr_1fr_1fr_auto] items-center gap-6 px-5 py-3.5";

function UsageRow({ usage, last }: { usage: UsageLine; last: boolean }) {
	const pct = Math.min(100, (usage.usedGb / usage.includedGb) * 100);
	const over = usage.usedGb > usage.includedGb;
	return (
		<div
			className={cn(ROW_CLASS, !last && "border-b border-foreground/10")}
		>
			<RowTitle
				icon={usage.icon}
				title={usage.title}
				description={usage.description}
			/>
			<div className="text-sm tabular-nums text-foreground">
				{usage.usedGb} GB
			</div>
			<div className="min-w-0">
				<div className="text-xs text-muted-foreground">
					of {usage.includedGb} GB included
				</div>
				<div className="relative mt-1 h-1 rounded-full bg-foreground/10">
					<div
						className={cn(
							"absolute h-1 rounded-full",
							over ? "bg-amber-500" : "bg-primary",
						)}
						style={{ width: `${pct}%` }}
					/>
				</div>
			</div>
			<div className="text-right">
				<div className="text-sm font-medium tabular-nums text-foreground">
					{formatCurrency(usage.overageUsd)}
				</div>
				<div className="text-[11px] text-muted-foreground">
					this period
				</div>
			</div>
		</div>
	);
}

// The control plane is fixed by the plan: nothing to meter, so show what it
// runs in place of a usage bar.
function ControlPlaneRow({ q }: { q: Quote }) {
	const { nodes, size } = q.controlPlane;
	return (
		<div className={cn(ROW_CLASS, "border-b border-foreground/10")}>
			<RowTitle
				icon={faServer}
				title="Control plane"
				description={`Preconfigured by the ${q.tier.label} plan.`}
			/>
			<div className="text-sm tabular-nums text-foreground">
				{nodes} × {size.label}
			</div>
			<div className="text-xs text-muted-foreground">
				{formatVcpu(size.vcpu.value * nodes)} vCPU ·{" "}
				{formatMemory(size.memoryMb * nodes)} total
			</div>
			<div className="text-right">
				<div className="text-sm font-medium tabular-nums text-foreground">
					{q.tier.monthlyUsd === 0
						? "Free"
						: formatCurrency(q.tier.monthlyUsd)}
				</div>
				<div className="text-[11px] text-muted-foreground">
					fixed per month
				</div>
			</div>
		</div>
	);
}

// Runner compute is billed per node, not per second, so it is a count × size
// rather than a usage bar.
function RunnerComputeRow({ q }: { q: Quote }) {
	const { count, size } = q.runners;
	return (
		<div className={cn(ROW_CLASS, "border-b border-foreground/10")}>
			<RowTitle
				icon={faServer}
				title="Runner compute"
				description="Actors run on runner nodes. Scale in Settings."
			/>
			<div className="text-sm tabular-nums text-foreground">
				{count} × {size.label}
			</div>
			<div className="text-xs text-muted-foreground">
				{formatVcpu(size.vcpu.value * count)} vCPU ·{" "}
				{formatMemory(size.memoryMb * count)} total
			</div>
			<div className="text-right">
				<div className="text-sm font-medium tabular-nums text-foreground">
					{q.cloud.id === "byoc"
						? "Your cloud"
						: q.tier.id === "free"
							? "Included"
							: formatCurrency(q.runnersUsd)}
				</div>
				<div className="text-[11px] text-muted-foreground">
					per month
				</div>
			</div>
		</div>
	);
}

function RowTitle({
	icon,
	title,
	description,
}: {
	icon: IconProp;
	title: string;
	description: string;
}) {
	return (
		<div className="flex min-w-0 items-start gap-3">
			<div className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md border border-foreground/10">
				<Icon icon={icon} className="size-3.5" />
			</div>
			<div className="min-w-0">
				<div className="text-sm font-medium text-foreground">
					{title}
				</div>
				<div className="truncate text-xs text-muted-foreground">
					{description}
				</div>
			</div>
		</div>
	);
}
