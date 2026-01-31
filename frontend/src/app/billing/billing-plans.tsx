import { Rivet } from "@rivet-gg/cloud";
import { useMutation, useQuery, useSuspenseQuery } from "@tanstack/react-query";
import type { ComponentProps } from "react";
import { Button, type ButtonProps } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";
import { CommunityPlan, EnterprisePlan, ProPlan, TeamPlan } from "./plan-card";

export function BillingPlans() {
	const dataProvider = useCloudProjectDataProvider();

	const {
		data: { billing },
	} = useSuspenseQuery(
		dataProvider.currentProjectBillingDetailsQueryOptions(),
	);

	const { mutate, isPending, variables } = useMutation({
		...dataProvider.changeCurrentProjectBillingPlanMutationOptions(),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.currentProjectBillingDetailsQueryOptions(),
			);
			await queryClient.invalidateQueries({
				predicate(query) {
					return (
						query.queryKey[1] ===
						dataProvider.currentProjectLatestMetricsQueryOptions({
							name: [],
							namespace: "",
						}).queryKey[1]
					);
				},
			});
		},
	});

	const { refetch: createUpdateSubscriptionSession, data } = useQuery({
		...dataProvider.currentProjectBillingSubscriptionUpdateSessionQueryOptions(),
	});

	return (
		<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4 mt-4">
			{(
				[
					[Rivet.BillingPlan.Free, CommunityPlan],
					[Rivet.BillingPlan.Pro, ProPlan],
					[Rivet.BillingPlan.Team, TeamPlan],
				] as const
			).map(([plan, PlanComponent]) => {
				const config = getConfig(plan, billing);
				return (
					<PlanComponent
						key={plan}
						{...config}
						buttonProps={{
							...config.buttonProps,
							disabled: config.buttonProps.disabled || isPending,
							isLoading: variables?.__from === plan && isPending,
							hidden: config.buttonProps.hidden,
							onMouseEnter: () => {
								createUpdateSubscriptionSession();
							},
							onClick: () => {
								if (!billing.canChangePlan) {
									return window.open(data?.url, "_blank");
								}
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
						window.open("https://www.rivet.dev/sales", "_blank");
					},
				}}
			/>
		</div>
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
	billing: Rivet.BillingDetailsResponse.Billing,
) {
	return {
		current: isCurrent(plan, billing),
		buttonProps: {
			children: buttonText(plan, billing),
			variant: buttonVariant(plan, billing),
			disabled: buttonDisabled(plan, billing),
			hidden: buttonHidden(plan, billing),
		},
	};
}

function buttonVariant(
	plan: Rivet.BillingPlan,
	data: Rivet.BillingDetailsResponse.Billing,
): ButtonProps["variant"] {
	if (plan === data.activePlan && data.futurePlan !== data.activePlan)
		return "default";
	if (plan === data.futurePlan && data.futurePlan !== data.activePlan)
		return "secondary";

	if (comparePlans(plan, data.futurePlan || Rivet.BillingPlan.Free) > 0)
		return "default";
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
	if (plan === data.activePlan && data.futurePlan !== data.activePlan) {
		return <>Resubscribe</>;
	}
	if (plan === data.futurePlan && data.futurePlan !== data.activePlan) {
		if (!data.currentPeriodEnd) {
			return null;
		}
		return (
			<>
				Downgrades on{" "}
				{new Date(data.currentPeriodEnd).toLocaleDateString(undefined, {
					month: "short",
					day: "numeric",
				})}
			</>
		);
	}
	if (plan === data.activePlan) {
		return "Current Plan";
	}

	return comparePlans(plan, data.futurePlan || Rivet.BillingPlan.Free) > 0
		? "Upgrade"
		: "Downgrade";
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
	];

	const tierA = plans.indexOf(a);
	const tierB = plans.indexOf(b);

	if (tierA > tierB) return 1;
	if (tierA < tierB) return -1;
	return 0;
}

export function CurrentPlan({ plan }: { plan?: string }) {
	if (!plan || plan === "free") return <>Free</>;
	if (plan === "pro") return <>Hobby</>;
	if (plan === "team") return <>Team</>;
	return <>Enterprise</>;
}

export function BillingDetailsButton(props: ComponentProps<typeof Button>) {
	const dataProvider = useCloudProjectDataProvider();

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
