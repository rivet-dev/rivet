import { faQuestionCircle, Icon } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { endOfMonth, startOfMonth } from "date-fns";
import { BillingPlans } from "@/app/billing/billing-plans";
import { BillingStatus } from "@/app/billing/billing-status";
import { ComputeUsageCard } from "@/app/billing/compute-card";
import { CurrentBillTotal } from "@/app/billing/current-bill-card";
import { useBilledComputeCost } from "@/app/billing/hooks";
import { ManageBillingButton } from "@/app/billing/manage-billing-button";
import { UsageCard } from "@/app/billing/usage-card";
import { USAGE_METRICS } from "@/app/billing/usage-metrics";
import { HelpDropdown } from "@/app/help-dropdown";
import { Button, H1 } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { computeOutsideTotalUsd } from "@/content/billing";
import { features } from "@/lib/features";
import { Content } from "../layout";

/**
 * Namespace billing page. Billing is always project-scoped, so this renders the
 * exact same project billing as the project route.
 */
export function NamespaceBillingPage() {
	return <BillingPage />;
}

export function BillingPage() {
	return (
		<Content>
			<div className="mb-4 pt-2 max-w-5xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4 ">
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

			<BillingBody />
		</Content>
	);
}

/**
 * Headerless billing content (no SidebarToggle / H1 / Help). Safe to render
 * outside `RootLayoutContextProvider`, e.g. inside the settings drawer.
 */
export function BillingBody() {
	const dataProvider = useCloudProjectDataProvider();
	// Single backend call returns the fully-computed breakdown; the page just
	// renders it (no client-side pricing math).
	const { data: usage } = useSuspenseQuery({
		...dataProvider.currentProjectBillingUsageQueryOptions(),
	});
	const compute = useBilledComputeCost();

	const metricsByKey = new Map(usage.metrics.map((m) => [m.metric, m]));
	const computeDollars = compute.isError ? 0 : compute.monthToDate;

	return (
		<div className="px-4  max-w-5xl mx-auto @6xl:px-0 space-y-8 pb-8">
			<CurrentBillTotal
				total={
					Number(usage.totalCents) / 100 +
					computeOutsideTotalUsd(usage.plan, computeDollars)
				}
				periodStart={
					usage.currentPeriodStart
						? new Date(usage.currentPeriodStart)
						: startOfMonth(new Date())
				}
				periodEnd={
					usage.currentPeriodEnd
						? new Date(usage.currentPeriodEnd)
						: endOfMonth(new Date())
				}
			/>
			<BillingPlansSection />
			{USAGE_METRICS.map(
				({ key, title, description, icon, metricType }) => {
					const m = metricsByKey.get(key);
					return (
						<UsageCard
							key={key}
							title={title}
							description={description}
							current={BigInt(m?.usage ?? 0)}
							monthToDate={BigInt(m?.overageCents ?? 0)}
							includedInPlan={
								m && m.included > 0
									? BigInt(m.included)
									: undefined
							}
							icon={icon}
							metricType={metricType}
						/>
					);
				},
			)}
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
