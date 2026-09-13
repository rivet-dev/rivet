import type { Rivet } from "@rivet-gg/cloud";
import { createFileRoute } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { useState } from "react";
import z from "zod";
import { SelectPlanScreen } from "@/app/billing/select-plan";
import CreateClusterFrameContent from "@/app/dialogs/create-cluster-frame";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { SidebarlessHeader } from "@/app/layout";
import { Card } from "@/components";
import { features } from "@/lib/features";
import { TEST_IDS } from "@/utils/test-ids";

export const Route = createFileRoute("/_context/orgs/$organization/new/")({
	component: RouteComponent,
	validateSearch: zodValidator(
		z.object({
			flow: z.enum(["agent", "manual"]).optional(),
			modal: z.string().optional(),
			showAll: z.coerce.boolean().optional(),
		}),
	),
});

function RouteComponent() {
	const search = Route.useSearch();
	const navigate = Route.useNavigate();
	const params = Route.useParams();

	// Billing-enabled flavors ask for the plan first; the choice is applied
	// after the project is created. Skipping falls back to Free. Choosing
	// BYOC branches into cluster creation instead of a project.
	const [step, setStep] = useState<"plan" | "details" | "byoc">(
		features.billing ? "plan" : "details",
	);
	const [plan, setPlan] = useState<Rivet.BillingPlan | undefined>(undefined);

	return (
		<div className="h-screen flex flex-col overflow-hidden">
			<SidebarlessHeader />
			{step === "plan" ? (
				<SelectPlanScreen
					onSelect={(selected) => {
						setPlan(selected);
						setStep("details");
					}}
					onSelectByoc={() => setStep("byoc")}
					onSkip={() => {
						setPlan(undefined);
						setStep("details");
					}}
				/>
			) : step === "byoc" ? (
				<div className="flex-1 min-h-0 flex mx-auto w-full px-6 items-center justify-center overflow-auto">
					<div className="max-w-2xl w-full py-6">
						<Card className="max-w-2xl w-full">
							<CreateClusterFrameContent
								organization={params.organization}
							/>
						</Card>
					</div>
				</div>
			) : (
				<div className="flex-1 min-h-0 flex mx-auto w-full px-6 items-center justify-center overflow-auto">
					<div className="max-w-2xl w-full py-6">
						<Card
							className="max-w-2xl w-full"
							data-testid={TEST_IDS.Onboarding.CreateProjectCard}
						>
							<CreateProjectFrameContent
								organization={params.organization}
								plan={plan}
								onSuccess={(data, vars) => {
									if (vars.namespace) {
										return navigate({
											to: "/orgs/$organization/projects/$project/ns/$namespace",
											params: {
												organization: vars.organization,
												project: data.project.name,
												namespace: vars.namespace,
											},
											search: {
												flow: search.flow,
											},
										});
									}

									return navigate({
										to: "/orgs/$organization/projects/$project",
										params: {
											organization: vars.organization,
											project: data.project.name,
										},
										search: {
											flow: search.flow,
										},
									});
								}}
							/>
						</Card>
					</div>
				</div>
			)}
		</div>
	);
}
