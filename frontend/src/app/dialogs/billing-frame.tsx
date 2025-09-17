import {
	useMutation,
	useQuery,
	useSuspenseQueries,
} from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import type { ComponentProps } from "react";
import { Button, DocsSheet, Frame, Link } from "@/components";
import { queryClient } from "@/queries/global";
import {
	CommunityPlan,
	EnterprisePlan,
	ProPlan,
	TeamPlan,
} from "../billing/plan-card";

export default function BillingFrameContent() {
	const { dataProvider } = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project",
	});

	const [
		{ data: project },
		{
			data: { billing },
		},
	] = useSuspenseQueries({
		queries: [
			dataProvider.currentProjectQueryOptions(),
			dataProvider.currentProjectBillingDetailsQueryOptions(),
		],
	});

	const { mutate, isPending, variables } = useMutation({
		...dataProvider.changeCurrentProjectBillingPlanMutationOptions(),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.currentProjectBillingDetailsQueryOptions(),
			);
		},
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title>{project.name} billing</Frame.Title>
				<Frame.Description>
					Manage billing for your Rivet Cloud project.{" "}
					<DocsSheet
						path="https://www.rivet.gg/pricing"
						title="Billing"
					>
						<Link className="cursor-pointer">
							Learn more about billing.
						</Link>
					</DocsSheet>
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<div className="flex justify-between items-center border rounded-md p-4">
					<div>
						<p>
							You are currently on the{" "}
							<span className="font-semibold">
								<CurrentPlan plan={billing?.activePlan} />
							</span>{" "}
							plan.{" "}
							{billing?.futurePlan &&
							billing.activePlan !== billing?.futurePlan &&
							billing.currentPeriodEnd ? (
								<>
									Your plan will change to{" "}
									<span className="font-semibold">
										<CurrentPlan
											plan={billing.futurePlan}
										/>
									</span>{" "}
									on{" "}
									{new Date(
										billing.currentPeriodEnd,
									).toLocaleDateString(undefined, {
										year: "numeric",
										month: "long",
										day: "numeric",
									})}
									.{" "}
								</>
							) : null}
							{!billing?.canChangePlan ? (
								// organization does not have a payment method, ask them to add one
								<span className="font-medium">
									You cannot change plans until you add a
									payment method to your organization.
								</span>
							) : null}
						</p>
					</div>

					<BillingDetailsButton variant="secondary">
						Manage billing details
					</BillingDetailsButton>
				</div>
				<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4 mt-4">
					<CommunityPlan
						current={
							billing?.activePlan === "free" ||
							!billing?.activePlan
						}
						buttonProps={{
							isLoading: isPending,
							onClick: () => mutate({ plan: "community" }),
							disabled:
								billing?.activePlan === "free" ||
								!billing?.activePlan ||
								!billing?.canChangePlan,
						}}
					/>
					<ProPlan
						current={billing?.activePlan === "pro"}
						buttonProps={{
							isLoading: isPending,
							onClick: () => {
								if (billing?.activePlan === "pro") {
									return mutate({ plan: "free" });
								}
								return mutate({ plan: "pro" });
							},
							disabled: !billing?.canChangePlan,
							...(billing?.activePlan === "pro" &&
							billing?.futurePlan !== "free"
								? { children: "Cancel" }
								: {}),
						}}
					/>
					<TeamPlan
						current={billing?.activePlan === "team"}
						buttonProps={{
							isLoading: isPending,
							onClick: () => {
								if (billing?.activePlan === "team") {
									return mutate({ plan: "free" });
								}
								return mutate({ plan: "team" });
							},
							disabled: !billing?.canChangePlan,
							...(billing?.activePlan === "team" &&
							billing?.futurePlan !== "free"
								? { children: "Cancel" }
								: {}),
						}}
					/>
					<EnterprisePlan />
				</div>
			</Frame.Content>
		</>
	);
}

function CurrentPlan({ plan }: { plan?: string }) {
	if (!plan || plan === "free") return <>Free</>;
	if (plan === "pro") return <>Hobby</>;
	if (plan === "team") return <>Team</>;
	return <>Enterprise</>;
}

function BillingDetailsButton(props: ComponentProps<typeof Button>) {
	const { dataProvider } = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project",
	});

	const { data, refetch } = useQuery(
		dataProvider.billingCustomerPortalSessionQueryOptions(),
	);

	return (
		<Button
			{...props}
			onMouseEnter={() => {
				refetch();
			}}
			onClick={() => {
				if (data) {
					window.open(data, "_blank");
				}
			}}
		/>
	);
}
