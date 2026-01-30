import {
	faBarcodeRead,
	faDatabase,
	faPencil,
	faQuestionCircle,
	faRunning,
	faSignalStream,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { endOfMonth, startOfMonth } from "date-fns";
import { BillingPlans } from "@/app/billing/billing-plans";
import { BillingStatus } from "@/app/billing/billing-status";
import { CurrentBillTotal } from "@/app/billing/current-bill-card";
import { useAggregatedMetrics } from "@/app/billing/hooks";
import { ManageBillingButton } from "@/app/billing/manage-billing-button";
import { type MetricType, UsageCard } from "@/app/billing/usage-card";
import { HelpDropdown } from "@/app/help-dropdown";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { Button, H1 } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { BILLING } from "@/content/billing";
import { Content } from "../layout";

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
		title: "Awake Actors",
		description: "Time your actors spend running and processing requests.",
		icon: faRunning,
		metricType: "hours",
	},
	{
		key: "kv_storage_used",
		title: "State Storage",
		description:
			"Persistent data stored in actor state across all namespaces.",
		icon: faDatabase,
		metricType: "bytes",
	},
	{
		key: "kv_read",
		title: "Reads",
		description: "Data read from actor state, measured in 4KB units.",
		icon: faBarcodeRead,
		metricType: "operations",
	},
	{
		key: "kv_write",
		title: "Writes",
		description: "Data written to actor state, measured in 4KB units.",
		icon: faPencil,
		metricType: "operations",
	},
	{
		key: "gateway_egress",
		title: "Egress",
		description:
			"Network traffic sent from your actors to external clients.",
		icon: faSignalStream,
		metricType: "bytes",
	},
];

function calculateOverageCost(
	usage: bigint,
	includedInPlan: bigint | undefined,
	pricePerBillionUnits: bigint,
): bigint {
	if (!includedInPlan) return 0n;
	const overage = usage > includedInPlan ? usage - includedInPlan : 0n;
	return (overage * pricePerBillionUnits) / 1_000_000_000n;
}

export function BillingPage() {
	const dataProvider = useCloudProjectDataProvider();
	const { data } = useSuspenseQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});
	const metrics = useAggregatedMetrics();
	const plan = data?.billing.activePlan || "free";

	const totalOverageCents = USAGE_METRICS.reduce((total, { key }) => {
		const current = metrics[key] || 0n;
		const includedInPlan = BILLING.included[plan][key];
		return (
			total +
			calculateOverageCost(current, includedInPlan, BILLING.prices[key])
		);
	}, 0n);

	return (
		<Content>
			<div className="mb-4 pt-2 max-w-5xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4 ">
					<SidebarToggle className="absolute left-4" />
					<H1>Billing</H1>
					<HelpDropdown>
						<Button
							variant="outline"
							startIcon={<Icon icon={faQuestionCircle} />}
						>
							Need help?
						</Button>
					</HelpDropdown>
				</div>
				<p className="max-w-5xl mb-6 px-6 @6xl:px-0 text-muted-foreground">
					Manage your project's billing information and view usage
					details.
				</p>
			</div>

			<hr className="mb-6" />

			<div className="px-4  max-w-5xl mx-auto @6xl:px-0 space-y-8 pb-8">
				<CurrentBillTotal
					total={Number(totalOverageCents) / 100}
					periodStart={
						data.billing.currentPeriodStart
							? new Date(data.billing.currentPeriodStart)
							: startOfMonth(new Date())
					}
					periodEnd={
						data.billing.currentPeriodEnd
							? new Date(data.billing.currentPeriodEnd)
							: endOfMonth(new Date())
					}
				/>
				<BillingPlansSection />
				{USAGE_METRICS.map(
					({ key, title, description, icon, metricType }) => {
						const current = metrics[key] || 0n;
						const includedInPlan = BILLING.included[plan][key];
						return (
							<UsageCard
								key={key}
								title={title}
								description={description}
								current={current}
								monthToDate={calculateOverageCost(
									current,
									includedInPlan,
									BILLING.prices[key],
								)}
								includedInPlan={includedInPlan}
								icon={icon}
								metricType={metricType}
							/>
						);
					},
				)}
			</div>
		</Content>
	);
}

function BillingPlansSection() {
	return (
		<>
			<div className="border p-4 rounded-md flex justify-between items-center">
				<BillingStatus />
				<ManageBillingButton>Manage billing</ManageBillingButton>
			</div>
			<BillingPlans />
		</>
	);
}
