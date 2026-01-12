import { Rivet } from "@rivet-gg/cloud";
import {
	useMutation,
	useQuery,
	useSuspenseQueries,
} from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import type { ComponentProps } from "react";
import { Button, Frame, Link } from "@/components";
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
				<Frame.Title>{project.displayName} billing</Frame.Title>
				<Frame.Description>
					Manage billing for your Rivet Cloud project.{" "}
					<Link asChild className="cursor-pointer">
						<a
							target="_blank"
							rel="noopener noreferrer"
							href="https://www.rivet.dev/pricing"
						>
							Learn more about billing.
						</a>
					</Link>
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
								<span className="font-bold text-destructive">
									You cannot change plans until you add a
									payment method to your organization.
								</span>
							) : null}
						</p>
					</div>

					<BillingDetailsButton
						variant={
							billing?.canChangePlan ? "secondary" : "default"
						}
					>
						Manage billing details
					</BillingDetailsButton>
				</div>
				<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4 mt-4">
					{[
						[Rivet.BillingPlan.Free, CommunityPlan],
						[Rivet.BillingPlan.Pro, ProPlan],
						[Rivet.BillingPlan.Team, TeamPlan],
					].map(([plan, PlanComponent]) => {
						const config = getConfig(plan, billing);
						return (
							<PlanComponent
								key={plan}
								{...config}
								buttonProps={{
									...config.buttonProps,
									disabled:
										config.buttonProps.disabled ||
										isPending,
									isLoading:
										variables?.__from === plan && isPending,
									hidden: config.buttonProps.hidden,
									onClick: () => {
										if (billing.futurePlan === plan) {
											return mutate({
												plan: Rivet.BillingPlan.Free,
												__from: plan,
											});
										}
										mutate({ plan, __from: plan });
									},
								}}
							/>
						);
					})}
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
			</Frame.Content>
		</>
	);
}

function isCurrent(
	plan: Rivet.BillingPlan,
	data: Rivet.BillingDetailsResponse.Billing,
) {
	return (
		plan === data.activePlan ||
		(plan === Rivet.BillingPlan.Free && !data.activePlan)
	);
}

function getConfig(
	plan: Rivet.BillingPlan,
	billing: Rivet.BillingDetailsResponse.Billing | undefined,
) {
	return {
		current: isCurrent(plan, billing),
		buttonProps: {
			children: buttonText(plan, billing),
			variant: buttonVariant(plan, billing),
			disabled: !billing?.canChangePlan || buttonDisabled(plan, billing),
			hidden: buttonHidden(plan, billing),
		},
	};
}

function buttonVariant(
	plan: Rivet.BillingPlan,
	data: Rivet.BillingDetailsResponse.Billing,
) {
	if (plan === data.activePlan && data.futurePlan !== data.activePlan)
		return "default";
	if (plan === data.futurePlan && data.futurePlan !== data.activePlan)
		return "secondary";

	if (comparePlans(plan, data.futurePlan) > 0) return "default";
	return "secondary";
}

function buttonDisabled(
	plan: Rivet.BillingPlan,
	data: Rivet.BillingDetailsResponse.Billing,
) {
	return plan === data.futurePlan && data.futurePlan !== data.activePlan;
}

function buttonText(
	plan: Rivet.BillingPlan,
	data: Rivet.BillingDetailsResponse.Billing,
) {
	if (plan === data.activePlan && data.futurePlan !== data.activePlan)
		return <>Resubscribe</>;
	if (plan === data.futurePlan && data.futurePlan !== data.activePlan)
		return (
			<>
				Downgrades on{" "}
				{new Date(data.currentPeriodEnd).toLocaleDateString(undefined, {
					month: "short",
					day: "numeric",
				})}
			</>
		);
	if (plan === data.activePlan) return "Cancel";
	return comparePlans(plan, data.futurePlan) > 0 ? "Upgrade" : "Downgrade";
}

function buttonHidden(
	plan: Rivet.BillingPlan,
	data: Rivet.BillingDetailsResponse.Billing,
) {
	return plan === data.activePlan && plan === Rivet.BillingPlan.Free;
}

export function comparePlans(
	a: Rivet.BillingPlan,
	b: Rivet.BillingPlan,
): number {
	const plans = [
		Rivet.BillingPlan.Free,
		Rivet.BillingPlan.Pro,
		Rivet.BillingPlan.Team,
		Rivet.BillingPlan.Enterprise,
	];

	const tierA = plans.indexOf(a);
	const tierB = plans.indexOf(b);

	if (tierA > tierB) return 1;
	if (tierA < tierB) return -1;
	return 0;
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
