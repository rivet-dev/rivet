import type { Story } from "@ladle/react";
import "../../../.ladle/ladle.css";
import { TooltipProvider } from "@/components";
import { PLAN_LABELS } from "@/content/billing";
import { PlanBadge } from "./billing-plan-badge";
import { CommunityPlan, EnterprisePlan, ProPlan, TeamPlan } from "./plan-card";

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<TooltipProvider>
			<div className="bg-background min-h-screen p-8 text-foreground">
				{children}
			</div>
		</TooltipProvider>
	);
}

// Every plan side by side. Billing data is cloud-only, so this is the only
// place all four plan colors can be compared at once.
export const Badges: Story = () => (
	<Frame>
		<div className="flex items-center gap-3">
			{Object.keys(PLAN_LABELS).map((plan) => (
				<PlanBadge key={plan} plan={plan} />
			))}
		</div>
	</Frame>
);

export const Cards: Story = () => (
	<Frame>
		<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
			<CommunityPlan current buttonProps={{ children: "Current plan" }} />
			<ProPlan buttonProps={{ children: "Upgrade" }} />
			<TeamPlan buttonProps={{ children: "Upgrade" }} />
			<EnterprisePlan />
		</div>
	</Frame>
);
