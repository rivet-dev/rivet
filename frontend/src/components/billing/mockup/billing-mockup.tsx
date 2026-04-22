import {
	faArrowUpRight,
	faBarcodeRead,
	faDatabase,
	faInfoCircle,
	faPencil,
	faRunning,
	faSignalStream,
	faSparkles,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { useState } from "react";
import {
	Button,
	cn,
	Dialog,
	DialogContent,
	DialogTitle,
} from "@/components";

// -- Plan Summary Card --

function PlanSummaryCard({ onUpgrade }: { onUpgrade: () => void }) {
	return (
		<div className="rounded-xl border dark:border-white/10 bg-card overflow-hidden">
			<div className="grid grid-cols-1 lg:grid-cols-[1fr_auto]">
				<div className="p-6 lg:p-8">
					<div className="flex items-center gap-2">
						<span className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/50 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
							Current plan
						</span>
					</div>
					<div className="mt-3 flex items-baseline gap-2">
						<h2 className="text-2xl font-semibold text-foreground">
							Free
						</h2>
						<span className="text-sm text-muted-foreground">
							$0/mo
						</span>
					</div>
					<p className="mt-1.5 text-sm text-muted-foreground max-w-md">
						Perfect for exploring Rivet. Upgrade anytime to unlock
						more capacity and support.
					</p>
					<div className="mt-5 flex items-center gap-2">
						<Button size="sm" className="gap-1.5" onClick={onUpgrade}>
							<Icon icon={faSparkles} className="w-3" />
							Upgrade plan
						</Button>
						<Button variant="ghost" size="sm" className="gap-1.5">
							Manage billing
							<Icon icon={faArrowUpRight} className="w-3" />
						</Button>
					</div>
				</div>

				<div className="border-t lg:border-t-0 lg:border-l border-border dark:border-white/10 bg-muted/20 p-6 lg:p-8 lg:w-[320px]">
					<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
						<span>Current bill</span>
						<Icon
							icon={faInfoCircle}
							className="text-[10px] opacity-60"
						/>
					</div>
					<div className="mt-1 text-3xl font-semibold tabular-nums text-foreground">
						$0.00
					</div>
					<div className="mt-4">
						<div className="flex items-center justify-between text-xs">
							<span className="text-muted-foreground">
								Billing period
							</span>
							<span className="font-medium text-foreground">
								13 days left
							</span>
						</div>
						<div className="mt-2 h-1 rounded-full bg-border overflow-hidden">
							<div
								className="h-full rounded-full bg-foreground/60 dark:bg-foreground/50"
								style={{ width: "56%" }}
							/>
						</div>
						<div className="mt-2 flex items-center justify-between text-[11px] text-muted-foreground tabular-nums">
							<span>Apr 3</span>
							<span>May 3</span>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}

// -- Plan Cards --

interface PlanFeature {
	label: string;
	emphasized?: boolean;
}

interface Plan {
	name: string;
	tagline: string;
	price: string;
	priceSuffix?: string;
	usageBased?: boolean;
	custom?: boolean;
	current?: boolean;
	popular?: boolean;
	features: PlanFeature[];
	ctaLabel: string;
	ctaVariant?: "default" | "outline" | "secondary";
}

const PLANS: Plan[] = [
	{
		name: "Free",
		tagline: "For exploring and side projects.",
		price: "$0",
		priceSuffix: "/mo",
		current: true,
		features: [
			{ label: "5M writes /mo" },
			{ label: "200M reads /mo" },
			{ label: "5 GiB storage" },
			{ label: "100 GiB egress" },
			{ label: "100K awake actor hours" },
			{ label: "Community support" },
		],
		ctaLabel: "Current plan",
		ctaVariant: "secondary",
	},
	{
		name: "Hobby",
		tagline: "For growing apps.",
		price: "$20",
		priceSuffix: "/mo",
		usageBased: true,
		features: [
			{ label: "25B reads /mo included", emphasized: true },
			{ label: "50M writes /mo included", emphasized: true },
			{ label: "5 GiB storage included" },
			{ label: "1 TiB egress included" },
			{ label: "400K awake actor hours" },
			{ label: "Email support" },
		],
		ctaLabel: "Upgrade",
	},
	{
		name: "Team",
		tagline: "For production workloads.",
		price: "$200",
		priceSuffix: "/mo",
		usageBased: true,
		popular: true,
		features: [
			{ label: "Everything in Hobby" },
			{ label: "MFA required" },
			{ label: "Slack support" },
			{ label: "Team management" },
			{ label: "Higher rate limits" },
			{ label: "Priority queue" },
		],
		ctaLabel: "Upgrade",
	},
	{
		name: "Enterprise",
		tagline: "For mission-critical systems.",
		price: "Custom",
		custom: true,
		features: [
			{ label: "Everything in Team" },
			{ label: "Priority support" },
			{ label: "Uptime SLA" },
			{ label: "OIDC SSO" },
			{ label: "Audit logs" },
			{ label: "Custom roles" },
			{ label: "Volume pricing" },
		],
		ctaLabel: "Contact us",
		ctaVariant: "outline",
	},
];

function PlanCard({ plan }: { plan: Plan }) {
	return (
		<div
			className={cn(
				"relative rounded-xl border dark:border-white/10 bg-card p-5 flex flex-col transition-all",
				"hover:border-foreground/20 dark:hover:border-white/20",
				plan.current && "ring-1 ring-primary/40 border-primary/30",
				plan.popular &&
					!plan.current &&
					"ring-1 ring-foreground/10 dark:ring-white/10",
			)}
		>
			{plan.current ? (
				<span className="absolute -top-2 left-5 rounded-full bg-primary text-primary-foreground text-[10px] font-semibold tracking-wide uppercase px-2 py-0.5 shadow-sm">
					Current
				</span>
			) : plan.popular ? (
				<span className="absolute -top-2 left-5 rounded-full border border-border bg-card text-foreground text-[10px] font-semibold tracking-wide uppercase px-2 py-0.5 shadow-sm">
					Popular
				</span>
			) : null}

			<div>
				<h3 className="text-base font-semibold text-foreground">
					{plan.name}
				</h3>
				<p className="text-xs text-muted-foreground mt-0.5">
					{plan.tagline}
				</p>
			</div>

			<div className="mt-5 min-h-[60px]">
				<div className="flex items-baseline gap-1">
					<span className="text-3xl font-semibold tracking-tight text-foreground tabular-nums">
						{plan.price}
					</span>
					{plan.priceSuffix ? (
						<span className="text-sm text-muted-foreground">
							{plan.priceSuffix}
						</span>
					) : null}
				</div>
				{plan.usageBased ? (
					<p className="mt-1 text-xs text-muted-foreground">
						+ Usage-based pricing
					</p>
				) : plan.custom ? (
					<p className="mt-1 text-xs text-muted-foreground">
						Tailored to your team
					</p>
				) : (
					<p className="mt-1 text-xs text-muted-foreground">
						Free forever
					</p>
				)}
			</div>

			<ul className="mt-5 space-y-2 flex-1">
				{plan.features.map((feature) => (
					<li
						key={feature.label}
						className={cn(
							"flex items-start gap-2 text-xs",
							feature.emphasized
								? "text-foreground"
								: "text-muted-foreground",
						)}
					>
						<span
							className={cn(
								"mt-[5px] size-1 rounded-full shrink-0",
								feature.emphasized
									? "bg-foreground"
									: "bg-muted-foreground/50",
							)}
						/>
						<span className="leading-relaxed">{feature.label}</span>
					</li>
				))}
			</ul>

			<Button
				variant={plan.ctaVariant ?? "default"}
				size="sm"
				className="w-full mt-6"
				disabled={plan.current}
			>
				{plan.ctaLabel}
			</Button>
		</div>
	);
}

function UpgradePlanDialog({
	open,
	onOpenChange,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
}) {
	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				className="max-w-5xl p-6 sm:p-8 rounded-xl"
				hideClose
			>
				<div className="flex items-baseline justify-between gap-4">
					<div>
						<DialogTitle className="text-base font-semibold text-foreground">
							Plans
						</DialogTitle>
						<p className="text-xs text-muted-foreground mt-0.5">
							Pick the plan that fits your workload. Change
							anytime.
						</p>
					</div>
					<a
						href="#pricing"
						className="text-xs text-muted-foreground hover:text-foreground inline-flex items-center gap-1 transition-colors shrink-0"
					>
						Compare plans in detail
						<Icon icon={faArrowUpRight} className="w-2.5" />
					</a>
				</div>
				<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
					{PLANS.map((plan) => (
						<PlanCard key={plan.name} plan={plan} />
					))}
				</div>
			</DialogContent>
		</Dialog>
	);
}

// -- Usage Section --

interface UsageMetric {
	key: string;
	title: string;
	description: string;
	icon: IconProp;
	current: string;
	included: string;
	percent: number;
	monthToDate: string;
}

const USAGE_METRICS: UsageMetric[] = [
	{
		key: "actor_awake",
		title: "Awake actors",
		description: "Time your actors spend running and processing requests.",
		icon: faRunning,
		current: "0 hrs",
		included: "10M hrs",
		percent: 0,
		monthToDate: "$0.00",
	},
	{
		key: "storage",
		title: "State storage",
		description: "Persistent data stored in actor state.",
		icon: faDatabase,
		current: "0 B",
		included: "5 GiB",
		percent: 0,
		monthToDate: "$0.00",
	},
	{
		key: "reads",
		title: "Reads",
		description: "Data read from actor state, in 4KB units.",
		icon: faBarcodeRead,
		current: "0 ops",
		included: "200M ops",
		percent: 0,
		monthToDate: "$0.00",
	},
	{
		key: "writes",
		title: "Writes",
		description: "Data written to actor state, in 4KB units.",
		icon: faPencil,
		current: "0 ops",
		included: "5M ops",
		percent: 0,
		monthToDate: "$0.00",
	},
	{
		key: "egress",
		title: "Egress",
		description: "Network traffic sent from actors to clients.",
		icon: faSignalStream,
		current: "0 B",
		included: "100 GiB",
		percent: 0,
		monthToDate: "$0.00",
	},
];

function UsageRow({ metric }: { metric: UsageMetric }) {
	return (
		<div className="grid grid-cols-[1fr_1fr_auto] items-center gap-6 px-5 py-4 hover:bg-muted/20 transition-colors">
			<div className="flex items-center gap-3 min-w-0">
				<div className="flex h-8 w-8 items-center justify-center rounded-md border border-border dark:border-white/10 bg-background/50 shrink-0">
					<Icon
						icon={metric.icon}
						className="text-sm text-muted-foreground"
					/>
				</div>
				<div className="min-w-0">
					<div className="text-sm font-medium text-foreground truncate">
						{metric.title}
					</div>
					<div className="text-xs text-muted-foreground truncate">
						{metric.description}
					</div>
				</div>
			</div>

			<div className="min-w-0">
				<div className="flex items-baseline justify-between gap-2 text-xs">
					<span className="tabular-nums font-medium text-foreground">
						{metric.current}
					</span>
					<span className="tabular-nums text-muted-foreground">
						of {metric.included}
					</span>
				</div>
				<div className="mt-1.5 h-1 rounded-full bg-border overflow-hidden">
					<div
						className={cn(
							"h-full rounded-full transition-all",
							metric.percent > 80
								? "bg-destructive"
								: metric.percent > 50
									? "bg-yellow-500"
									: "bg-primary",
						)}
						style={{ width: `${Math.max(metric.percent, 1)}%` }}
					/>
				</div>
			</div>

			<div className="text-right tabular-nums shrink-0 w-20">
				<div className="text-sm font-semibold text-foreground">
					{metric.monthToDate}
				</div>
				<div className="text-[10px] text-muted-foreground">
					this period
				</div>
			</div>
		</div>
	);
}

function UsageSection() {
	return (
		<div>
			<div className="flex items-baseline justify-between mb-4">
				<div>
					<h2 className="text-sm font-semibold text-foreground">
						Usage
					</h2>
					<p className="text-xs text-muted-foreground mt-0.5">
						Current billing period usage vs. plan limits.
					</p>
				</div>
				<span className="text-xs text-muted-foreground tabular-nums">
					Updated just now
				</span>
			</div>
			<div className="rounded-xl border dark:border-white/10 bg-card divide-y divide-border dark:divide-white/10">
				{USAGE_METRICS.map((metric) => (
					<UsageRow key={metric.key} metric={metric} />
				))}
			</div>
		</div>
	);
}

// -- Drawer body --

export function BillingContent() {
	const [upgradeOpen, setUpgradeOpen] = useState(false);

	return (
		<div className="space-y-8 pb-10">
			<PlanSummaryCard onUpgrade={() => setUpgradeOpen(true)} />
			<UsageSection />
			<UpgradePlanDialog
				open={upgradeOpen}
				onOpenChange={setUpgradeOpen}
			/>
		</div>
	);
}
