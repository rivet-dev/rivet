import {
	faBarcodeRead,
	faDatabase,
	faPencil,
	faQuestionCircle,
	faRunning,
	faSignalStream,
	Icon,
} from "@rivet-gg/icons";
import { createFileRoute } from "@tanstack/react-router";
import { BillingPlans } from "@/app/billing/billing-plans";
import { BillingStatus } from "@/app/billing/billing-status";
import { CurrentBillTotal } from "@/app/billing/current-bill-card";
import { ManageBillingButton } from "@/app/billing/manage-billing-button";
import { UsageCard } from "@/app/billing/usage-card";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { Button, H1 } from "@/components";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/billing",
)({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<RouteLayout>
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
					<CurrentBillingSection />
					<BillingPlansSection />
					<AwakeActorsUsageSection />
					<StateStorageUsageSection />
					<ReadsOnlyBillingSection />
					<WritesOnlyBillingSection />
					<EgressBillingSection />
				</div>
			</Content>
		</RouteLayout>
	);
}

function CurrentBillingSection() {
	return (
		<CurrentBillTotal
			total={123.45}
			periodStart={new Date("2024-05-01")}
			periodEnd={new Date("2026-05-31")}
		/>
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

function AwakeActorsUsageSection() {
	return (
		<UsageCard
			title="Awake Actors"
			description="Monitor your Awake Actors usage to manage costs effectively."
			current={80000}
			monthToDate={75000}
			includedInPlan={100000}
			icon={faRunning}
		/>
	);
}

function StateStorageUsageSection() {
	return (
		<UsageCard
			title="State Storage"
			description="Monitor your State Storage usage to manage costs effectively."
			current={1500}
			monthToDate={1500}
			includedInPlan={1000}
			icon={faDatabase}
		/>
	);
}

function ReadsOnlyBillingSection() {
	return (
		<UsageCard
			title="Reads"
			description="Monitor your Reads usage to manage costs effectively."
			current={80000}
			monthToDate={75000}
			includedInPlan={90000}
			icon={faBarcodeRead}
		/>
	);
}

function WritesOnlyBillingSection() {
	return (
		<UsageCard
			title="Writes"
			description="Monitor your Writes usage to manage costs effectively."
			current={80000}
			monthToDate={75000}
			includedInPlan={100000}
			icon={faPencil}
		/>
	);
}

function EgressBillingSection() {
	return (
		<UsageCard
			title="Egress"
			description="Monitor your Egress usage to manage costs effectively."
			current={80000}
			monthToDate={75000}
			includedInPlan={100000}
			icon={faSignalStream}
		/>
	);
}
