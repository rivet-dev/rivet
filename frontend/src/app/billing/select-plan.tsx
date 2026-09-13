import { Rivet } from "@rivet-gg/cloud";
import { faArrowRight, Icon } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { Content } from "@/app/layout";
import { Button } from "@/components";
import { TEST_IDS } from "@/utils/test-ids";
import { CommunityPlan, EnterprisePlan, ProPlan, TeamPlan } from "./plan-card";

/**
 * Full-screen plan selection shown as the first step of project creation,
 * before the project exists. Purely presentational: the chosen plan is
 * applied after the project is created. Mirrors the getting-started layout:
 * the caller renders it under a SidebarlessHeader inside an h-screen column.
 */
export function SelectPlanScreen({
	onSelect,
	onSelectByoc,
	onSkip,
}: {
	onSelect: (plan: Rivet.BillingPlan) => void;
	onSelectByoc: () => void;
	onSkip: () => void;
}) {
	return (
		<Content className="flex-1 min-h-0 !h-auto !overflow-hidden flex flex-col items-center justify-safe-center">
			<motion.div
				className="relative min-w-0 w-full flex-1 min-h-0 flex flex-col"
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.3 }}
				data-testid={TEST_IDS.Onboarding.SelectPlan}
			>
				<div className="absolute top-2 right-2 z-10">
					<Button
						variant="ghost"
						size="sm"
						className="text-muted-foreground hover:text-foreground"
						endIcon={<Icon icon={faArrowRight} className="ms-1" />}
						onClick={() => onSkip()}
					>
						Skip for now
					</Button>
				</div>
				<div className="flex-1 min-h-0 overflow-auto flex items-safe-center justify-center px-4 py-8">
					<div className="w-full max-w-6xl">
						<div className="text-center mb-8">
							<h1 className="text-2xl font-semibold">
								Choose a plan
							</h1>
							<p className="text-muted-foreground mt-1">
								You can change it at any time in billing
								settings.
							</p>
						</div>
						<div className="@container">
							<div className="grid grid-cols-1 @xl:grid-cols-2 @5xl:grid-cols-4 gap-4">
								<CommunityPlan
									current
									buttonProps={{
										children: "Select",
										variant: "secondary",
										onClick: () =>
											onSelect(Rivet.BillingPlan.Free),
									}}
								/>
								<ProPlan
									buttonProps={{
										children: "Select",
										onClick: () =>
											onSelect(Rivet.BillingPlan.Pro),
									}}
								/>
								<TeamPlan
									buttonProps={{
										children: "Select",
										onClick: () =>
											onSelect(Rivet.BillingPlan.Team),
									}}
								/>
								<EnterprisePlan
									buttonProps={{
										onClick: () => {
											window.open(
												"https://www.rivet.dev/sales",
												"_blank",
											);
										},
									}}
								/>
							</div>
							<div className="my-6 border-t" />
							<ByocCard onSelect={() => onSelectByoc()} />
						</div>
					</div>
				</div>
			</motion.div>
		</Content>
	);
}

function ByocCard({ onSelect }: { onSelect: () => void }) {
	return (
		<div className="border rounded-lg p-6 hover:bg-secondary/20 transition-colors flex flex-col @2xl:flex-row @2xl:items-center gap-4">
			<div className="flex-1">
				<h3 className="text-lg font-medium">Bring Your Own Cloud</h3>
				<p className="text-sm text-muted-foreground mt-1">
					Run Rivet on your own infrastructure. Deploy to your cloud
					account and keep full control over where your workloads
					run.
				</p>
			</div>
			<Button
				variant="secondary"
				className="@2xl:w-auto w-full"
				onClick={() => onSelect()}
			>
				Select
			</Button>
		</div>
	);
}
